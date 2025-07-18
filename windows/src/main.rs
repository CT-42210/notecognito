use anyhow::{Context, Result};
use notecognito_core::{ConfigManager, NotecardId};
use std::sync::Arc;
use tokio::sync::Mutex;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder,
};
use windows::Win32::{
    Foundation::*,
    UI::WindowsAndMessaging::*,
};

mod hotkey;
mod ipc_client;
mod notecard_window;
mod platform_impl;

use hotkey::HotkeyManager;
use ipc_client::IpcClient;
use notecard_window::NotecardWindowManager;
use platform_impl::WindowsPlatform;

const APP_NAME: &str = "Notecognito";
const WM_USER_TRAY: u32 = WM_USER + 1;

struct App {
    config_manager: Arc<Mutex<ConfigManager>>,
    ipc_client: Arc<Mutex<IpcClient>>,
    hotkey_manager: Arc<Mutex<HotkeyManager>>,
    window_manager: Arc<Mutex<NotecardWindowManager>>,
    platform: Arc<Mutex<WindowsPlatform>>,
    tray_icon: Option<TrayIcon>,
}

impl App {
    async fn new() -> Result<Self> {
        // Initialize logging
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();

        tracing::info!("Starting Notecognito for Windows");

        // Create config manager
        let config_manager = ConfigManager::new()
            .context("Failed to create config manager")?;
        let config_manager = Arc::new(Mutex::new(config_manager));

        // Create IPC client and connect
        let ipc_client = IpcClient::new();
        let ipc_client = Arc::new(Mutex::new(ipc_client));

        // Create managers
        let hotkey_manager = Arc::new(Mutex::new(HotkeyManager::new()));
        let window_manager = Arc::new(Mutex::new(NotecardWindowManager::new()));

        // Create platform implementation
        let platform = WindowsPlatform::new(
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
            tray_icon: None,
        })
    }

    async fn initialize(&mut self) -> Result<()> {
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

        // Load configuration and setup hotkeys
        self.load_configuration().await?;

        // Create system tray
        self.create_system_tray()?;

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
            hotkey_manager.register_hotkey(notecard_id, modifiers)?;
        }

        // Set launch on startup
        if config.launch_on_startup {
            self.set_launch_on_startup(true).await?;
        }

        Ok(())
    }

    fn create_system_tray(&mut self) -> Result<()> {
        // Load tray icon
        let icon_bytes = include_bytes!("../assets/icon.ico");
        let icon = image::load_from_memory(icon_bytes)?;

        // Create tray menu
        let show_config = MenuItem::new("Configure", true, None);
        let separator = PredefinedMenuItem::separator();
        let quit = MenuItem::new("Quit", true, None);

        let menu = Menu::new();
        menu.append(&show_config)?;
        menu.append(&separator)?;
        menu.append(&quit)?;

        // Create tray icon
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(APP_NAME)
            .with_icon(tray_icon::Icon::from_rgba(
                icon.to_rgba8().into_raw(),
                icon.width(),
                icon.height(),
            )?)
            .build()?;

        self.tray_icon = Some(tray_icon);

        // Handle menu events
        let show_config_id = show_config.id();
        let quit_id = quit.id();

        tokio::spawn(async move {
            let menu_channel = MenuEvent::receiver();
            while let Ok(event) = menu_channel.recv() {
                if event.id == show_config_id {
                    Self::launch_config_ui();
                } else if event.id == quit_id {
                    std::process::exit(0);
                }
            }
        });

        Ok(())
    }

    fn launch_config_ui() {
        // Launch the Electron configuration UI
        let config_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .map(|p| p.join("notecognito-config.exe"))
            .unwrap_or_else(|| "notecognito-config.exe".into());

        if let Err(e) = std::process::Command::new(&config_path).spawn() {
            tracing::error!("Failed to launch config UI: {}", e);
        }
    }

    async fn set_launch_on_startup(&self, enabled: bool) -> Result<()> {
        use windows::Win32::System::Registry::*;

        unsafe {
            let key_path = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
            let mut hkey = HKEY::default();

            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                key_path,
                0,
                KEY_SET_VALUE,
                &mut hkey,
            )?;

            if enabled {
                let exe_path = std::env::current_exe()?;
                let exe_path = exe_path.to_string_lossy();
                let value = format!("\"{}\"", exe_path);

                RegSetValueExW(
                    hkey,
                    w!("Notecognito"),
                    0,
                    REG_SZ,
                    Some(value.as_bytes()),
                )?;
            } else {
                let _ = RegDeleteValueW(hkey, w!("Notecognito"));
            }

            RegCloseKey(hkey)?;
        }

        Ok(())
    }

    async fn run(&mut self) -> Result<()> {
        // Set up hotkey message handler
        let config_manager = Arc::clone(&self.config_manager);
        let window_manager = Arc::clone(&self.window_manager);

        {
            let mut hotkey_manager = self.hotkey_manager.lock().await;

            hotkey_manager.start_message_loop(move |notecard_id| {
                let config_manager = Arc::clone(&config_manager);
                let window_manager = Arc::clone(&window_manager);

                // Use a separate runtime for the callback
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async move {
                        if let Err(e) = show_notecard(notecard_id, config_manager, window_manager).await {
                            tracing::error!("Failed to show notecard: {}", e);
                        }
                    });
                });
            })?;
        }

        // Keep the main thread alive
        // The hotkey message loop runs in a separate thread
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            // Check if we should exit (this could be triggered by a shutdown event)
            if self.should_exit().await {
                break;
            }
        }

        Ok(())
    }

    async fn should_exit(&self) -> bool {
        // This could check for a shutdown flag set by the tray menu
        // For now, we'll rely on process termination
        false
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

#[tokio::main]
async fn main() -> Result<()> {
    // Check if already running
    let mutex_name = format!("Global\\{}", APP_NAME);
    unsafe {
        let mutex = CreateMutexW(None, true, &HSTRING::from(&mutex_name))?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            eprintln!("Notecognito is already running");
            return Ok(());
        }
    }

    // Create and run app
    let mut app = App::new().await?;
    app.initialize().await?;
    app.run().await?;

    Ok(())
}