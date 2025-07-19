use anyhow::{Context, Result};
use notecognito_core::{ConfigManager, NotecardId, PlatformInterface};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::ClassType;
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem,
    NSImage, NSEventModifierFlags,
};
use objc2_foundation::{
    MainThreadMarker, NSBundle, NSData, NSString,
};
use std::sync::Arc;
use tokio::sync::Mutex;

mod hotkey;
mod ipc_client;
mod notecard_window;
mod platform_impl;
mod app_delegate;

use hotkey::HotkeyManager;
use ipc_client::IpcClient;
use notecard_window::NotecardWindowManager;
use platform_impl::MacOSPlatform;
use app_delegate::AppDelegate;

const APP_NAME: &str = "Notecognito";

// Global references for menu items and delegate
static mut MENU_DELEGATE: Option<Retained<AppDelegate>> = None;
static mut STATUS_ITEM: Option<Retained<NSStatusItem>> = None;

pub struct App {
    config_manager: Arc<Mutex<ConfigManager>>,
    ipc_client: Arc<Mutex<IpcClient>>,
    hotkey_manager: Arc<Mutex<HotkeyManager>>,
    window_manager: Arc<Mutex<NotecardWindowManager>>,
    platform: Arc<Mutex<MacOSPlatform>>,
}

impl App {
    async fn new() -> Result<Self> {
        // Initialize logging
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();

        tracing::info!("Starting Notecognito for macOS");

        // Create config manager
        let config_manager = ConfigManager::new()
            .context("Failed to create config manager")?;
        let config_manager = Arc::new(Mutex::new(config_manager));

        // Create IPC client
        let ipc_client = IpcClient::new();
        let ipc_client = Arc::new(Mutex::new(ipc_client));

        // Create managers
        let hotkey_manager = Arc::new(Mutex::new(HotkeyManager::new()));
        let window_manager = Arc::new(Mutex::new(NotecardWindowManager::new()));

        // Create platform implementation
        let platform = MacOSPlatform::new(
            Arc::clone(&hotkey_manager),
            Arc::clone(&window_manager),
        );
        let platform = Arc::new(Mutex::new(platform));

        Ok(App {
            config_manager,
            ipc_client,
            hotkey_manager,
            window_manager,
            platform,
        })
    }

    async fn initialize(&mut self, mtm: MainThreadMarker) -> Result<()> {
        tracing::debug!("Initializing app...");

        // Set up as menu bar app (no dock icon)
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

        // Create and set app delegate FIRST
        let delegate = AppDelegate::new(mtm);
        unsafe {
            MENU_DELEGATE = Some(delegate.clone());
            app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        }

        // Create menu bar item AFTER delegate is set
        self.create_menu_bar_item(mtm)?;

        // Try to connect to IPC server
        match self.connect_to_core().await {
            Ok(_) => tracing::info!("Connected to core service"),
            Err(e) => {
                tracing::warn!("Could not connect to core service: {}", e);
                tracing::info!("Running in standalone mode");
            }
        }

        // Initialize platform
        {
            let mut platform = self.platform.lock().await;
            platform.initialize()?;
        }

        // Load configuration (but don't start hotkey monitoring yet)
        self.load_configuration_without_hotkeys().await?;

        // Now try to start hotkey monitoring, which will check permissions
        if let Err(e) = self.setup_hotkeys().await {
            tracing::warn!("Failed to setup hotkeys: {}", e);
            // Show permission alert if it's a permission issue
            if e.to_string().contains("Accessibility permissions") {
                self.show_accessibility_alert(mtm);
            }
        }

        Ok(())
    }


    fn show_accessibility_alert(&self, mtm: MainThreadMarker) {
        use objc2_app_kit::{NSAlert, NSAlertStyle};

        unsafe {
            let alert = NSAlert::new(mtm);
            alert.setMessageText(&NSString::from_str("Accessibility Permission Required"));
            alert.setInformativeText(&NSString::from_str(
                "Notecognito needs accessibility permissions to register global hotkeys.\n\n\
                Please grant permission in System Preferences > Security & Privacy > Privacy > Accessibility.\n\n\
                You may need to restart the app after granting permission."
            ));
            alert.setAlertStyle(NSAlertStyle::Warning);
            alert.runModal();
        }
    }

    async fn connect_to_core(&self) -> Result<()> {
        let mut client = self.ipc_client.lock().await;
        client.connect().await?;

        // Get configuration from core
        let config = client.get_configuration().await?;

        // Update local config
        let mut manager = self.config_manager.lock().await;
        *manager.config_mut() = config;

        Ok(())
    }


    fn create_menu_bar_item(&mut self, mtm: MainThreadMarker) -> Result<()> {
        tracing::debug!("Creating menu bar item...");

        unsafe {
            // Get the status bar
            let status_bar = NSStatusBar::systemStatusBar();

            // Create status item with variable length
            let status_item = status_bar.statusItemWithLength(-1.0); // NSVariableStatusItemLength

            // Set icon
            if let Some(button) = status_item.button(mtm) {
                // Try to load icon from bundle first
                if let Some(icon) = Self::load_icon(mtm) {
                    button.setImage(Some(&icon));
                    button.setToolTip(Some(&NSString::from_str("Notecognito")));
                } else {
                    // Fallback to text
                    button.setTitle(&NSString::from_str("N"));
                }
            }

            // Create menu with proper delegate target
            let menu = Self::create_menu(mtm);
            status_item.setMenu(Some(&menu));

            // Store status item globally
            STATUS_ITEM = Some(status_item);

            tracing::info!("Menu bar item created successfully");
        }

        Ok(())
    }

    fn load_icon(_mtm: MainThreadMarker) -> Option<Retained<NSImage>> {
        unsafe {
            // Try multiple ways to load the icon

            // 1. Try from app bundle resources
            let bundle = NSBundle::mainBundle();
            if let Some(path) = bundle.pathForResource_ofType(
                Some(&NSString::from_str("icon")),
                Some(&NSString::from_str("png"))
            ) {
                if let Some(image) = NSImage::initWithContentsOfFile(NSImage::alloc(), &path) {
                    // Set template image for proper menu bar styling
                    let _: () = objc2::msg_send![&image, setTemplate: true];
                    return Some(image);
                }
            }

            // 2. Try embedded icon data
            let icon_data = include_bytes!("../assets/icon.png");
            let data = NSData::dataWithBytes_length(
                icon_data.as_ptr() as *mut std::ffi::c_void,
                icon_data.len(),
            );

            if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
                // Set template image
                let _: () = objc2::msg_send![&image, setTemplate: true];
                return Some(image);
            }

            None
        }
    }

    fn create_menu(mtm: MainThreadMarker) -> Retained<NSMenu> {
        unsafe {
            let menu = NSMenu::new(mtm);

            // Get delegate reference
            let delegate = unsafe { MENU_DELEGATE.as_ref().unwrap() };

            // Configure item
            let configure_item = NSMenuItem::new(mtm);
            configure_item.setTitle(&NSString::from_str("Configure..."));
            configure_item.setAction(Some(objc2::sel!(configure:)));
            configure_item.setTarget(Some(delegate)); // Set proper target
            menu.addItem(&configure_item);

            // Separator
            menu.addItem(&NSMenuItem::separatorItem(mtm));

            // About item
            let about_item = NSMenuItem::new(mtm);
            about_item.setTitle(&NSString::from_str("About Notecognito"));
            about_item.setAction(Some(objc2::sel!(about:)));
            about_item.setTarget(Some(delegate)); // Set proper target
            menu.addItem(&about_item);

            // Separator
            menu.addItem(&NSMenuItem::separatorItem(mtm));

            // Quit item (this targets the app, not the delegate)
            let quit_item = NSMenuItem::new(mtm);
            quit_item.setTitle(&NSString::from_str("Quit Notecognito"));
            quit_item.setAction(Some(objc2::sel!(terminate:)));
            quit_item.setKeyEquivalent(&NSString::from_str("q"));
            quit_item.setKeyEquivalentModifierMask(NSEventModifierFlags::NSEventModifierFlagCommand);
            menu.addItem(&quit_item);

            menu
        }
    }

    async fn run(&mut self) -> Result<()> {
        // Create a channel for hotkey events
        let (tx, mut rx) = tokio::sync::mpsc::channel::<NotecardId>(32);

        // Set up hotkey callback with channel sender
        let callback = move |notecard_id: NotecardId| {
            tracing::info!("Hotkey pressed for notecard {}", notecard_id.value());

            // Just send the notecard ID through the channel
            // This is safe to do from any thread
            if let Err(e) = tx.try_send(notecard_id) {
                tracing::error!("Failed to send hotkey event: {}", e);
            }
        };

        // Start hotkey monitoring
        {
            let mut hotkey_manager = self.hotkey_manager.lock().await;
            match hotkey_manager.start_monitoring(callback) {
                Ok(_) => {
                    tracing::info!("Hotkey monitoring started successfully");
                }
                Err(e) => {
                    tracing::error!("Failed to start hotkey monitoring: {}", e);
                    // Don't return error - app can still run without hotkeys
                }
            }
        }

        // Spawn a task to handle hotkey events
        let config_manager = Arc::clone(&self.config_manager);
        let window_manager = Arc::clone(&self.window_manager);

        tokio::spawn(async move {
            while let Some(notecard_id) = rx.recv().await {
                if let Err(e) = show_notecard(notecard_id, config_manager.clone(), window_manager.clone()).await {
                    tracing::error!("Failed to show notecard: {}", e);
                }
            }
        });

        Ok(())
    }

    async fn load_configuration_without_hotkeys(&self) -> Result<()> {
        let manager = self.config_manager.lock().await;
        let config = manager.config();

        // Set launch on startup
        if config.launch_on_startup {
            let mut platform = self.platform.lock().await;
            platform.set_launch_on_startup(true)?;
        }

        Ok(())
    }

    async fn setup_hotkeys(&self) -> Result<()> {
        let manager = self.config_manager.lock().await;
        let config = manager.config();

        // Register hotkeys for all notecards
        let mut hotkey_manager = self.hotkey_manager.lock().await;
        let modifiers = &config.hotkey_modifiers;

        for i in 1..=9 {
            let notecard_id = NotecardId::new(i)?;
            if let Some(notecard) = manager.get_notecard(notecard_id) {
                if !notecard.content.is_empty() {
                    hotkey_manager.register_hotkey(notecard_id, modifiers)?;
                }
            }
        }

        Ok(())
    }
}

async fn show_notecard(
    notecard_id: NotecardId,
    config_manager: Arc<Mutex<ConfigManager>>,
    window_manager: Arc<Mutex<NotecardWindowManager>>,  // No underscore!
) -> Result<()> {
    let manager = config_manager.lock().await;

    if let Some(notecard) = manager.get_notecard(notecard_id) {
        if !notecard.content.is_empty() {
            let properties = &manager.config().default_display_properties;

            // Actually show the notecard window
            let mut window_manager = window_manager.lock().await;
            window_manager.show_notecard(notecard_id, &notecard.content, properties).await?;
        }
    }

    Ok(())
}

pub fn launch_config_ui() {
    tracing::info!("Launching configuration UI...");

    // Try multiple approaches to launch the config UI

    // 1. First try using 'open' command with app name
    let result = std::process::Command::new("open")
        .args(&["-a", "Notecognito Config"])
        .spawn();

    if result.is_ok() {
        return;
    }

    // 2. Try looking in the same directory as our executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let config_app = parent.join("Notecognito Config.app");
            if config_app.exists() {
                if let Ok(_) = std::process::Command::new("open")
                    .arg(config_app)
                    .spawn()
                {
                    return;
                }
            }
        }
    }

    // 3. Try Applications folder
    let apps_path = "/Applications/Notecognito Config.app";
    if std::path::Path::new(apps_path).exists() {
        if let Ok(_) = std::process::Command::new("open")
            .arg(apps_path)
            .spawn()
        {
            return;
        }
    }

    tracing::error!("Failed to launch configuration UI - app not found");
}

#[tokio::main]
async fn main() -> Result<()> {
    // Get main thread marker
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| anyhow::anyhow!("Must be run on main thread"))?;

    // Create app instance
    let mut app = App::new().await?;

    // Initialize on main thread
    app.initialize(mtm).await?;

    // Start async tasks (but don't fail if hotkeys can't be registered)
    if let Err(e) = app.run().await {
        tracing::error!("Error during app run: {}", e);
        // Continue anyway - the app can still function
    }

    // Run the main event loop
    unsafe {
        let ns_app = NSApplication::sharedApplication(mtm);
        tracing::info!("Starting NSApplication run loop");
        ns_app.run();
    }

    Ok(())
}
