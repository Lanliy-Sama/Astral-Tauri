//! Astral 桌面应用 —— Tauri 2 库入口（桌面 + 移动端共用）。
//! 窗口参数对应原版 window_manager.dart：960×540，最小 200×300，无系统标题栏。

use tauri::{Manager, WindowEvent};

pub mod commands;

/// closeMinimize 共享状态（由前端 set_close_minimize 同步）
pub struct CloseMinimizeState(pub std::sync::Mutex<bool>);

/// 托盘隐藏命令（前端确认对话框通过后调用）
/// 移动端没有托盘，仅保留命令入口以维持前端调用一致。
#[tauri::command]
fn destroy_tray(app: tauri::AppHandle) {
    #[cfg(desktop)]
    {
        if let Some(tray) = app.tray_by_id("main-tray") {
            let _ = tray.set_visible(false);
        }
        if let Some(win) = app.get_webview_window("main") {
            let _ = win.hide();
        }
    }
    #[cfg(mobile)]
    let _ = &app;
}

/// 系统托盘（对应原版 tray_manager 菜单：显示主界面 / 隐藏托盘 / 退出）
#[cfg(desktop)]
fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{MenuBuilder, MenuItemBuilder};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let show = MenuItemBuilder::with_id("tray_show_window", "显示主界面").build(app)?;
    let hide_tray = MenuItemBuilder::with_id("tray_hide", "隐藏托盘").build(app)?;
    let exit = MenuItemBuilder::with_id("tray_exit", "退出").build(app)?;
    let menu = MenuBuilder::new(app)
        .item(&show)
        .separator()
        .item(&hide_tray)
        .separator()
        .item(&exit)
        .build()?;

    let tray = TrayIconBuilder::with_id("main-tray")
        .icon(app.default_window_icon().cloned().unwrap())
        .tooltip("Astral")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "tray_show_window" => {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            // 隐藏托盘：转交前端弹确认对话框（对应原版 trayHide 确认流程）
            "tray_hide" => {
                use tauri::Emitter;
                let _ = app.emit("tray-hide-request", ());
            }
            "tray_exit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle().clone();
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
        })
        .build(app)?;
    let _ = tray;
    Ok(())
}

#[tauri::command]
fn set_close_minimize(app: tauri::AppHandle, value: bool) {
    let state = app.state::<CloseMinimizeState>();
    let mut guard = state.0.lock().unwrap_or_else(|e| e.into_inner());
    *guard = value;
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    // v3 桌面端必须显式选择运行时（mobile 由 mobile_entry_point 注册）
    #[cfg(desktop)]
    let builder = builder.runtime(tauri_runtime_wry::Wry::default());

    builder
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_vpn_service::init())
        .manage(CloseMinimizeState(std::sync::Mutex::new(true)))
        .invoke_handler(tauri::generate_handler![
            commands::easytier_version,
            commands::get_running_info,
            commands::get_network_status,
            commands::create_server,
            commands::close_server,
            commands::set_tun_fd,
            commands::get_firewall_status,
            commands::set_firewall_status,
            commands::get_all_interfaces_metrics,
            commands::set_interface_metric,
            commands::test_network_connectivity,
            commands::start_magic_wall,
            commands::stop_magic_wall,
            commands::add_magic_wall_rule,
            commands::remove_magic_wall_rule,
            commands::update_magic_wall_rule,
            commands::get_magic_wall_status,
            commands::tcp_ping,
            commands::has_npcap_driver,
            commands::set_startup,
            commands::open_external,
            commands::vpn_prepare,
            commands::vpn_start,
            commands::vpn_stop,
            set_close_minimize,
            destroy_tray,
        ])
        .setup(|app| {
            // UDP 9999 日志桥（对应 Flutter LogCapture）
            commands::start_log_bridge(app.handle().clone());

            // 系统托盘（对应原版 tray_manager：显示主界面 / 隐藏托盘 / 退出）
            #[cfg(desktop)]
            setup_tray(app)?;

            // --autostart 参数：静默启动（隐藏窗口，对应 isAutostart/startupMinimize）
            let autostart = std::env::args().any(|a| a == "--autostart");
            if autostart {
                #[cfg(desktop)]
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.hide();
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // 关闭行为（对应 MainScreen.onWindowClose + closeMinimize 设置）
            #[cfg(desktop)]
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let state = app.state::<CloseMinimizeState>();
                let close_minimize = state.0.lock().map(|g| *g).unwrap_or(true);
                if close_minimize {
                    // 最小化到托盘：只隐藏窗口
                    let _ = window.hide();
                    api.prevent_close();
                }
                // 否则放行真正退出
            }
        })
        .run(tauri::generate_context!())
        .expect("Astral 启动失败");
}
