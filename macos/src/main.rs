use anyhow::{Context, Result};
use notecognito_core::{ConfigManager, NotecardId};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{msg_send_id, ClassType};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem,
    NSStatusBarButton, NSImage, NSEventModifierFlags,
};
use objc2_foundation::{
    ns_string, MainThreadMarker, NSBundle, NSData, NSNotificationCenter, NSObject, NSString,
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

pub struct App {
    config_manager: Arc<Mutex<ConfigManager>>,
    ipc_client: Arc<Mutex<IpcClient>>,
    hotkey_manager: Arc<Mutex<HotkeyManager>>,
    window_manager: Arc<Mutex<NotecardWindowManager>>,
    platform: Arc<Mutex<MacOSPlatform>>,
    status_item: Option<Retained<NSStatusItem>>,
}

impl App {
    async fn new() -> Result<Self> {
        // Initialize logging
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
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
            status_item: None,
        })
    }

    async fn initialize(&mut self, mtm: MainThreadMarker) -> Result<()> {
        // Set up as menu bar app (no dock icon)
        let app = NSApplication::sharedApplication(mtm);
        unsafe {
            app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
        }

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

        // Check permissions
        {
            let platform = self.platform.lock().await;
            if !platform.check_permissions()? {
                tracing::warn!("Accessibility permissions not granted");
                // The platform will have shown an alert
            }
        }

        // Load configuration and setup hotkeys
        self.load_configuration().await?;

        // Create menu bar item
        self.create_menu_bar_item(mtm)?;

        Ok(())
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

    async fn load_configuration(&self) -> Result<()> {
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

        // Set launch on startup
        if config.launch_on_startup {
            let mut platform = self.platform.lock().await;
            platform.set_launch_on_startup(true)?;
        }

        Ok(())
    }

    fn create_menu_bar_item(&mut self, mtm: MainThreadMarker) -> Result<()> {
        unsafe {
            // Get the status bar
            let status_bar = NSStatusBar::systemStatusBar();

            // Create status item
            let status_item = status_bar.statusItemWithLength(-1.0); // NSVariableStatusItemLength

            // Set icon
            if let Some(button) = status_item.button() {
                // Try to load icon from bundle
                if let Some(icon) = Self::load_icon(mtm) {
                    button.setImage(Some(&icon));
                } else {
                    // Fallback to text
                    button.setTitle(&NSString::from_str("N"));
                }
            }

            // Create menu
            let menu = Self::create_menu(mtm);
            status_item.setMenu(Some(&menu));

            self.status_item = Some(status_item);
        }

        Ok(())
    }

    fn load_icon(mtm: MainThreadMarker) -> Option<Retained<NSImage>> {
        unsafe {
            // Try to load from app bundle
            let bundle = NSBundle::mainBundle();
            if let Some(path) = bundle.pathForResource_ofType(Some(&NSString::from_str("icon")), Some(&NSString::from_str("png"))) {
                NSImage::initWithContentsOfFile(NSImage::alloc(), &path)
            } else {
                // Try to load embedded icon
                let icon_data = include_bytes!("../assets/icon.png");
                let data = NSData::dataWithBytes_length(
                    std::ptr::NonNull::new(icon_data.as_ptr() as *mut _).unwrap(),
                    icon_data.len(),
                );
                NSImage::initWithData(NSImage::alloc(), &data)
            }
        }
    }

    fn create_menu(mtm: MainThreadMarker) -> Retained<NSMenu> {
        unsafe {
            let menu = NSMenu::new(mtm);

            // Configure item
            let configure_item = NSMenuItem::new(mtm);
            configure_item.setTitle(&NSString::from_str("Configure..."));
            configure_item.setAction(Some(objc2::sel!(configure:)));
            configure_item.setTarget(Some(&NSApplication::sharedApplication(mtm).delegate().unwrap()));
            menu.addItem(&configure_item);

            // Separator
            menu.addItem(&NSMenuItem::separatorItem(mtm));

            // About item
            let about_item = NSMenuItem::new(mtm);
            about_item.setTitle(&NSString::from_str("About Notecognito"));
            about_item.setAction(Some(objc2::sel!(about:)));
            about_item.setTarget(Some(&NSApplication::sharedApplication(mtm).delegate().unwrap()));
            menu.addItem(&about_item);

            // Separator
            menu.addItem(&NSMenuItem::separatorItem(mtm));

            // Quit item
            let quit_item = NSMenuItem::new(mtm);
            quit_item.setTitle(&NSString::from_str("Quit"));
            quit_item.setAction(Some(objc2::sel!(terminate:)));
            quit_item.setKeyEquivalent(&NSString::from_str("q"));
            quit_item.setKeyEquivalentModifierMask(NSEventModifierFlags::NSEventModifierFlagCommand);
            menu.addItem(&quit_item);

            menu
        }
    }

    async fn run(&mut self) -> Result<()> {
        // Set up hotkey.rs callback
        let config_manager = Arc::clone(&self.config_manager);
        let window_manager = Arc::clone(&self.window_manager);

        let callback = move |notecard_id: NotecardId| {
            let config_manager = Arc::clone(&config_manager);
            let window_manager = Arc::clone(&window_manager);

            // Show notecard in async context
            tokio::spawn(async move {
                if let Err(e) = show_notecard(notecard_id, config_manager, window_manager).await {
                    tracing::error!("Failed to show notecard: {}", e);
                }
            });
        };

        // Start hotkey.rs monitoring
        {
            let mut hotkey_manager = self.hotkey_manager.lock().await;
            hotkey_manager.start_monitoring(callback)?;
        }

        Ok(())
    }
}

async fn show_notecard(
    notecard_id: NotecardId,
    config_manager: Arc<Mutex<ConfigManager>>,
    window_manager: Arc<Mutex<NotecardWindowManager>>,
) -> Result<()> {
    let manager = config_manager.lock().await;

    if let Some(notecard) = manager.get_notecard(notecard_id) {
        if !notecard.content.is_empty() {
            let properties = &manager.config().default_display_properties;
            let mut window_manager = window_manager.lock().await;
            window_manager.show_notecard(notecard_id, &notecard.content, properties)?;
        }
    }

    Ok(())
}

pub fn launch_config_ui() {
    // Launch the Electron configuration UI
    let config_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .map(|p| p.join("Notecognito Config.app/Contents/MacOS/Notecognito Config"))
        .unwrap_or_else(|| "open -a 'Notecognito Config'".into());

    if config_path.to_string_lossy().starts_with("open") {
        // Use open command
        if let Err(e) = std::process::Command::new("open")
            .args(&["-a", "Notecognito Config"])
            .spawn()
        {
            tracing::error!("Failed to launch config UI: {}", e);
        }
    } else {
        // Direct launch
        if let Err(e) = std::process::Command::new(&config_path).spawn() {
            tracing::error!("Failed to launch config UI: {}", e);
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Get main thread marker
    let mtm = MainThreadMarker::new().unwrap();

    // Create app instance
    let mut app = App::new().await?;

    // Initialize on main thread
    app.initialize(mtm).await?;

    // Create and set app delegate
    let delegate = AppDelegate::new(mtm);
    unsafe {
        let ns_app = NSApplication::sharedApplication(mtm);
        ns_app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    }

    // Start async tasks
    app.run().await?;

    // Run the main event loop
    unsafe {
        let ns_app = NSApplication::sharedApplication(mtm);
        ns_app.run();
    }

    Ok(())
}