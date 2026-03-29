//! AI Agent OS - Tauri Main Entry Point
//!
//! Native GUI application with global hotkeys and system tray integration.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::{
    CustomMenuItem, GlobalShortcutManager, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu,
    WindowBuilder, WindowUrl,
};
use tracing_subscriber::{fmt, EnvFilter};

mod lib;
use lib::*;

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!("🤖 Starting AI Agent OS...");

    // Create system tray
    let tray_menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("show".to_string(), "Show Dashboard"))
        .add_item(CustomMenuItem::new("analyze".to_string(), "Analyze System"))
        .add_native_item(tauri::SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("quit".to_string(), "Quit"));

    let system_tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .manage(AppState::new())
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::MenuItemClick { id, .. } => {
                match id.as_str() {
                    "show" => {
                        if let Some(window) = app.get_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "analyze" => {
                        tracing::info!("Triggering system analysis from tray...");
                        // Could emit an event here for the frontend to handle
                    }
                    "quit" => {
                        std::process::exit(0);
                    }
                    _ => {}
                }
            }
            SystemTrayEvent::LeftClick { .. } => {
                // Toggle window on tray click
                if let Some(window) = app.get_window("main") {
                    match window.is_visible() {
                        Ok(true) => {
                            let _ = window.hide();
                        }
                        Ok(false) => {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            init_agent,
            get_metrics,
            get_agent_state,
            analyze_system,
            execute_command,
            toggle_window,
            set_opacity,
            get_recent_problems,
            set_autonomy_level,
        ])
        .setup(|app| {
            // Get the main window
            let main_window = app.get_window("main").unwrap();

            // Register global shortcuts
            let mut shortcut_manager = app.global_shortcut_manager();

            // Super+Space: Toggle dashboard
            {
                let window = main_window.clone();
                shortcut_manager
                    .register("Super+Space", move || {
                        tracing::info!("🔥 Super+Space triggered - Toggle dashboard");
                        match window.is_visible() {
                            Ok(true) => {
                                let _ = window.hide();
                            }
                            Ok(false) => {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                            _ => {}
                        }
                    })
                    .expect("Failed to register Super+Space hotkey");
            }

            // Super+Shift+A: Trigger analysis
            {
                let window = main_window.clone();
                shortcut_manager
                    .register("Super+Shift+A", move || {
                        tracing::info!("🔍 Super+Shift+A triggered - System analysis");
                        // Emit event to frontend
                        let _ = window.emit("trigger-analysis", ());
                    })
                    .expect("Failed to register Super+Shift+A hotkey");
            }

            // Super+Shift+X: Screen capture (future vision analysis)
            {
                let window = main_window.clone();
                shortcut_manager
                    .register("Super+Shift+X", move || {
                        tracing::info!("📸 Super+Shift+X triggered - Screen capture");
                        let _ = window.emit("screen-capture", ());
                    })
                    .expect("Failed to register Super+Shift+X hotkey");
            }

            tracing::info!("✅ Global hotkeys registered:");
            tracing::info!("   Super+Space      - Toggle dashboard");
            tracing::info!("   Super+Shift+A    - System analysis");
            tracing::info!("   Super+Shift+X    - Screen capture");

            // Start the intelligent agent in background
            let app_handle = app.handle();
            tauri::async_runtime::spawn(async move {
                let state: tauri::State<AppState> = app_handle.state();

                match init_agent(state).await {
                    Ok(msg) => tracing::info!("✅ {}", msg),
                    Err(e) => tracing::error!("❌ Failed to start agent: {}", e),
                }
            });

            // Setup Hyprland window rules (if on Hyprland)
            setup_hyprland_rules(&main_window);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Configure Hyprland-specific window rules
fn setup_hyprland_rules(window: &tauri::Window) {
    // Get window class/title for Hyprland rules
    // These rules should ideally be set in Hyprland config, but we can
    // also try to apply them programmatically

    tracing::info!("🪟 Configuring Hyprland window rules...");

    // Try to execute hyprctl commands
    let commands = vec![
        // Float the window
        "hyprctl dispatch togglefloating",
        // Pin to all workspaces
        "hyprctl dispatch pin",
    ];

    for cmd in commands {
        if let Err(e) = std::process::Command::new("sh").arg("-c").arg(cmd).output() {
            tracing::warn!("Could not execute Hyprland command: {} - {}", cmd, e);
        }
    }

    tracing::info!("✅ Hyprland rules applied (best effort)");
}
