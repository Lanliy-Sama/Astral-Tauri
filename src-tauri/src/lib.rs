//! Astral 桌面应用 —— Tauri 2 库入口（桌面 + 移动端共用）。
//! 窗口参数对应原版 window_manager.dart：960×540，最小 200×300，无系统标题栏。

use tauri::{Manager, WindowEvent};

pub mod commands;

/// closeMinimize 共享状态（由前端 set_close_minimize 同步）
pub struct CloseMinimizeState(pub std::sync::Mutex<bool>);

/* ============================================================
 * 自定义背景（v2.6）：磁盘文件 + astralbg:// 自定义协议
 * ------------------------------------------------------------
 * 背景：v2.5 及更早把背景转成 base64 data URL 存 SQLite 并写进 CSS
 * 变量 —— Chromium 对 CSS 自定义属性值存在 ~1.5MB 上限，超过即被
 * 静默丢弃（背景纹丝不动），且巨型字符串在 IPC/SQLite/内存里全线膨胀。
 * v2.6 改为：前端把原始字节按 1MB 分块 base64 经 IPC 流式写入应用
 * 数据目录（bg_write_* 命令），astralbg:// 协议直接从磁盘服务图像 ——
 * 无大小限制、内存占用恒定、GIF 自动逐帧播放、重启即恢复。
 * ============================================================ */

/// 背景写入会话（分块追加中）
struct BgWriteSession {
    file: std::fs::File,
    tmp: std::path::PathBuf,
    final_path: std::path::PathBuf,
}

/// 背景写入会话共享状态
pub struct BgWriteState(pub std::sync::Mutex<Option<BgWriteSession>>);

/// 自定义背景支持的扩展名（与前端 BG_MIME_BY_EXT 对齐，均为 WebView 可渲染格式）
const BG_EXTS: &[&str] = &[
    "jpg", "jpeg", "jfif", "pjpe", "pjp", "png", "apng", "gif", "webp", "bmp", "dib",
    "svg", "ico", "cur", "tif", "tiff", "avif", "heic", "heif", "heics", "jxl",
];

/// 应用数据目录（背景文件所在，确保存在）
fn bg_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法定位应用数据目录: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建应用数据目录失败: {e}"))?;
    Ok(dir)
}

/// 扩展名 → Content-Type（astralbg 协议响应头）
fn bg_mime(ext: &str) -> &'static str {
    match ext {
        "jpg" | "jpeg" | "jfif" | "pjpe" | "pjp" => "image/jpeg",
        "png" => "image/png",
        "apng" => "image/apng",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" | "dib" => "image/bmp",
        "svg" => "image/svg+xml",
        "ico" | "cur" => "image/x-icon",
        "tif" | "tiff" => "image/tiff",
        "avif" => "image/avif",
        "heic" | "heics" => "image/heic",
        "heif" => "image/heif",
        "jxl" => "image/jxl",
        _ => "application/octet-stream",
    }
}

/// 当前背景文件（custom_background.<ext>，忽略 .tmp）
fn bg_current_file(dir: &std::path::Path) -> Option<(std::path::PathBuf, String)> {
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with("custom_background.") && !name.ends_with(".tmp") {
            let ext = name["custom_background.".len()..].to_ascii_lowercase();
            return Some((e.path(), ext));
        }
    }
    None
}

/// 背景版本号（mtime 毫秒）：换图后 URL 变化，绕过 WebView 缓存
fn bg_version(path: &std::path::Path) -> u128 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        })
        .unwrap_or(0)
}

/// astralbg:// 协议响应（磁盘直读，任意大小；缺失→ 404）
fn bg_response(dir: Option<&std::path::Path>) -> http::Response<Vec<u8>> {
    let mut builder = http::Response::builder()
        .header("Cache-Control", "no-store, no-cache, must-revalidate, max-age=0");
    match dir.and_then(bg_current_file) {
        Some((path, ext)) => match std::fs::read(&path) {
            Ok(bytes) => builder
                .header("Content-Type", bg_mime(&ext))
                .body(bytes)
                .unwrap(),
            Err(_) => builder.status(500).body(Vec::new()).unwrap(),
        },
        None => builder
            .status(http::StatusCode::NOT_FOUND)
            .header("Content-Type", "text/plain")
            .body(Vec::new())
            .unwrap(),
    }
}

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
                    let _ = win.unminimize();
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
                    let _ = win.unminimize();
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

/* ---------- 自定义背景命令（分块流式写入 + 查询 + 清除） ---------- */

/// 开始一次背景写入（ext 为小写扩展名；打断未完成的旧会话）
#[tauri::command]
fn bg_write_begin(app: tauri::AppHandle, ext: String) -> Result<(), String> {
    let ext = ext.to_ascii_lowercase();
    if !BG_EXTS.contains(&ext.as_str()) {
        return Err(format!("不支持的图像格式: .{ext}"));
    }
    let dir = bg_dir(&app)?;
    let tmp = dir.join("custom_background.tmp");
    let mut guard = app
        .state::<BgWriteState>()
        .0
        .lock()
        .map_err(|e| e.to_string())?;
    *guard = None; // 丢弃未完成的旧会话（句柄关闭）
    let file = std::fs::File::create(&tmp).map_err(|e| format!("创建临时文件失败: {e}"))?;
    *guard = Some(BgWriteSession {
        file,
        tmp,
        final_path: dir.join(format!("custom_background.{ext}")),
    });
    Ok(())
}

/// 追加一块 base64 数据（前端按 1MB 分块）
#[tauri::command]
fn bg_write_append(app: tauri::AppHandle, b64: String) -> Result<(), String> {
    use std::io::Write;
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.as_bytes())
        .map_err(|e| format!("base64 解码失败: {e}"))?;
    let mut guard = app
        .state::<BgWriteState>()
        .0
        .lock()
        .map_err(|e| e.to_string())?;
    let session = guard.as_mut().ok_or("背景写入会话未开始")?;
    session
        .file
        .write_all(&bytes)
        .map_err(|e| format!("写入背景数据失败: {e}"))?;
    Ok(())
}

/// 结束写入：落盘为正式背景文件（替换其它扩展名的旧背景），返回路径与版本
#[tauri::command]
fn bg_write_end(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let mut guard = app
        .state::<BgWriteState>()
        .0
        .lock()
        .map_err(|e| e.to_string())?;
    let session = guard.take().ok_or("背景写入会话未开始")?;
    let BgWriteSession { file, tmp, final_path } = session;
    drop(file); // 关闭句柄后再改名

    // 删除其它扩展名的旧背景（同扩展名由 rename 覆盖）
    if let Ok(entries) = std::fs::read_dir(bg_dir(&app)?) {
        for e in entries.flatten() {
            let path = e.path();
            if path != tmp && path != final_path {
                let name = e.file_name().to_string_lossy().into_owned();
                if name.starts_with("custom_background.") {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
    }
    std::fs::rename(&tmp, &final_path).map_err(|e| format!("背景文件落盘失败: {e}"))?;
    Ok(serde_json::json!({
        "path": final_path.to_string_lossy(),
        "version": bg_version(&final_path).to_string(),
    }))
}

/// 查询当前背景（None = 未设置）：{ path, ext, version }
#[tauri::command]
fn custom_background_info(app: tauri::AppHandle) -> Option<serde_json::Value> {
    let dir = app.path().app_data_dir().ok()?;
    bg_current_file(&dir).map(|(path, ext)| {
        serde_json::json!({
            "path": path.to_string_lossy(),
            "ext": ext,
            "version": bg_version(&path).to_string(),
        })
    })
}

/// 清除背景文件（恢复默认）
#[tauri::command]
fn clear_custom_background(app: tauri::AppHandle) -> Result<(), String> {
    let dir = bg_dir(&app)?;
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with("custom_background.") {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    Ok(())
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
        .manage(BgWriteState(std::sync::Mutex::new(None)))
        /* 自定义背景协议：astralbg://localhost/bg → 磁盘上的当前背景文件。
         * 异步 + 独立线程读盘：百 MB 级 GIF 也不阻塞主线程/渲染。 */
        .register_asynchronous_uri_scheme_protocol("astralbg", |ctx, _request, responder| {
            let dir = ctx.app_handle().path().app_data_dir().ok();
            std::thread::spawn(move || {
                responder.respond(bg_response(dir.as_deref()));
            });
        })
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
            bg_write_begin,
            bg_write_append,
            bg_write_end,
            custom_background_info,
            clear_custom_background,
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
