//! Stellaris Tauri 命令层 —— 把原 Flutter 版 rust/ 网络核心封装为 Tauri 2 命令。
//! 命令名与前端 `src/lib/rust/api.js` 的 invoke 调用一一对应；
//! 返回数据形状与浏览器 Mock 模式完全一致（camelCase）。

use astral_core::api::serde_types::*;
use astral_core::api::{firewall, hops, magic_wall, nat_test, simple};
use serde_json::Value;
use std::net::TcpStream;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

#[cfg(target_os = "android")]
use tauri_plugin_vpn_service::{models::StartVpnRequest, VpnserviceExt};

/* ==================== EasyTier 核心 ==================== */

#[tauri::command]
pub fn easytier_version() -> Result<String, String> {
    // 前端契约："版本号|平台名"（parsePlatformVersion 以 '|' 分隔出平台徽章）
    simple::easytier_version().map(|v| format!("{}|{}", v, std::env::consts::OS))
}

#[tauri::command]
pub async fn get_running_info() -> String {
    simple::get_running_info().await
}

#[tauri::command]
pub async fn get_running_info_for(instance_id: String) -> Result<String, String> {
    let id = uuid::Uuid::parse_str(&instance_id).map_err(|e| e.to_string())?;
    Ok(simple::get_running_info_for(id).await)
}

fn network_status_value(status: simple::KVNetworkStatus) -> Result<Value, String> {
    let dto = NetworkStatusDto {
        total_nodes: status.total_nodes as u64,
        nodes: status
            .nodes
            .iter()
            .map(|n| NodeInfoDto {
                peer_id: n.peer_id,
                hostname: n.hostname.clone(),
                ipv4: n.ipv4.clone(),
                latency_ms: n.latency_ms,
                nat: n.nat.clone(),
                hops: n
                    .hops
                    .iter()
                    .map(|h| NodeHopStatsDto {
                        peer_id: h.peer_id,
                        target_ip: h.target_ip.clone(),
                        latency_ms: h.latency_ms,
                        packet_loss: h.packet_loss,
                        node_name: h.node_name.clone(),
                    })
                    .collect(),
                loss_rate: n.loss_rate,
                connections: n
                    .connections
                    .iter()
                    .map(|c| NodeConnectionStatsDto {
                        conn_type: c.conn_type.clone(),
                        rx_bytes: c.rx_bytes,
                        tx_bytes: c.tx_bytes,
                        rx_packets: c.rx_packets,
                        tx_packets: c.tx_packets,
                    })
                    .collect(),
                tunnel_proto: n.tunnel_proto.clone(),
                conn_type: n.conn_type.clone(),
                rx_bytes: n.rx_bytes,
                tx_bytes: n.tx_bytes,
                version: n.version.clone(),
                cost: n.cost,
                proxy_cidrs: n.proxy_cidrs.clone(),
            })
            .collect(),
    };
    serde_json::to_value(dto).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_network_status() -> Result<Value, String> {
    network_status_value(simple::get_network_status().await)
}

#[tauri::command]
pub async fn get_network_status_for(instance_id: String) -> Result<Value, String> {
    let id = uuid::Uuid::parse_str(&instance_id).map_err(|e| e.to_string())?;
    network_status_value(simple::get_network_status_for(id).await)
}

fn flags_from_dto(flag: FlagsDto) -> simple::FlagsC {
    simple::FlagsC {
        default_protocol: flag.default_protocol,
        dev_name: flag.dev_name,
        enable_encryption: flag.enable_encryption,
        enable_ipv6: flag.enable_ipv6,
        mtu: flag.mtu,
        latency_first: flag.latency_first,
        enable_exit_node: flag.enable_exit_node,
        no_tun: flag.no_tun,
        use_smoltcp: flag.use_smoltcp,
        relay_network_whitelist: flag.relay_network_whitelist,
        disable_p2p: flag.disable_p2p,
        enable_udp_broadcast_relay: flag.enable_udp_broadcast_relay,
        relay_all_peer_rpc: flag.relay_all_peer_rpc,
        disable_udp_hole_punching: flag.disable_udp_hole_punching,
        disable_tcp_hole_punching: flag.disable_tcp_hole_punching,
        multi_thread: flag.multi_thread,
        data_compress_algo: flag.data_compress_algo,
        bind_device: flag.bind_device,
        enable_kcp_proxy: flag.enable_kcp_proxy,
        disable_kcp_input: flag.disable_kcp_input,
        disable_relay_kcp: flag.disable_relay_kcp,
        proxy_forward_by_system: flag.proxy_forward_by_system,
        accept_dns: flag.accept_dns,
        private_mode: flag.private_mode,
        enable_quic_proxy: flag.enable_quic_proxy,
        disable_quic_input: flag.disable_quic_input,
        disable_sym_hole_punching: flag.disable_sym_hole_punching,
        tcp_whitelist: flag.tcp_whitelist,
        udp_whitelist: flag.udp_whitelist,
        socks5_port: flag.socks5_port,
    }
}

fn start_server(config: CreateServerConfig) -> simple::JoinHandle<Result<uuid::Uuid, String>> {
    simple::cmd_log("command: create_server 进入");
    simple::send_udp_to_localhost("[cmd] create_server 进入");
    let flag = flags_from_dto(config.flag);
    simple::create_server(
        config.username,
        config.enable_dhcp,
        config.specified_ip,
        config.room_name,
        config.room_password,
        config.severurl,
        config.onurl,
        config.cidrs,
        Vec::new(),
        flag,
    )
}

/// 创建虚拟网络（fire-and-forget，与原版 JoinHandle 语义一致）。
#[tauri::command]
pub fn create_server(config: CreateServerConfig) -> Result<(), String> {
    let _ = start_server(config);
    simple::cmd_log("command: create_server 即将返回 Ok");
    simple::send_udp_to_localhost("[cmd] create_server 退出");
    Ok(())
}

/// 多房间模式：返回独立实例 UUID，供后续状态查询与关闭。
#[tauri::command]
pub async fn create_server_with_id(config: CreateServerConfig) -> Result<String, String> {
    let id = start_server(config)
        .await
        .map_err(|e| format!("创建实例任务失败: {e:#}"))??;
    Ok(id.to_string())
}

#[tauri::command]
pub fn close_server() {
    simple::cmd_log("command: close_server 进入");
    simple::send_udp_to_localhost("[cmd] close_server 进入");
    simple::close_server();
    simple::send_udp_to_localhost("[cmd] close_server 退出");
}

#[tauri::command]
pub async fn close_server_for(instance_id: String) -> Result<(), String> {
    let id = uuid::Uuid::parse_str(&instance_id).map_err(|e| e.to_string())?;
    simple::close_server_for(id).await
}

#[tauri::command]
pub async fn set_tun_fd(fd: i32) -> Result<(), String> {
    simple::set_tun_fd(fd).await
}

/* ==================== 防火墙 / 跃点 ==================== */

#[tauri::command]
pub fn get_firewall_status(profile_index: u32) -> Result<bool, String> {
    firewall::get_firewall_status(profile_index)
        .map_err(|e| format!("获取防火墙状态失败: {}", e))
}

#[tauri::command]
pub fn set_firewall_status(profile_index: u32, enable: bool) -> Result<(), String> {
    firewall::set_firewall_status(profile_index, enable)
        .map_err(|e| format!("设置防火墙状态失败: {}", e))
}

#[tauri::command]
pub fn get_all_interfaces_metrics() -> Vec<(String, u32)> {
    hops::get_all_interfaces_metrics().unwrap_or_default()
}

#[tauri::command]
pub fn set_interface_metric(interface_name: String, metric: u32) -> Result<(), String> {
    hops::set_interface_metric(&interface_name, metric).map_err(|e| e.to_string())
}

/* ==================== NAT 测试 ==================== */

#[tauri::command]
pub fn test_network_connectivity(stun_server: String) -> Result<Value, String> {
    let result = nat_test::test_network_connectivity(stun_server)?;
    let dto = NetworkTestResultDto {
        nat_type_v4: result.nat_type_v4,
        nat_type_v6: result.nat_type_v6,
        ipv4_latency: result.ipv4_latency,
        ipv6_latency: result.ipv6_latency,
    };
    serde_json::to_value(dto).map_err(|e| e.to_string())
}

/* ==================== 魔法墙 ==================== */

#[tauri::command]
pub fn start_magic_wall() -> Result<(), String> {
    magic_wall::start_magic_wall()
}

#[tauri::command]
pub fn stop_magic_wall() -> Result<(), String> {
    magic_wall::stop_magic_wall()
}

#[tauri::command]
pub fn add_magic_wall_rule(rule: magic_wall::MagicWallRule) -> Result<(), String> {
    magic_wall::add_magic_wall_rule(rule)
}

#[tauri::command]
pub fn remove_magic_wall_rule(rule_id: String) -> Result<(), String> {
    magic_wall::remove_magic_wall_rule(rule_id)
}

#[tauri::command]
pub fn update_magic_wall_rule(rule: magic_wall::MagicWallRule) -> Result<(), String> {
    magic_wall::update_magic_wall_rule(rule)
}

#[tauri::command]
pub fn get_magic_wall_status() -> Result<Value, String> {
    let status = magic_wall::get_magic_wall_status()?;
    serde_json::to_value(status).map_err(|e| e.to_string())
}

/* ==================== 平台工具 ==================== */

/// TCP ping（对应 Dart PingUtil）：超时/失败/>800ms 返回 null。
#[tauri::command]
pub async fn tcp_ping(url: String, timeout_ms: u64) -> Option<u32> {
    tauri::async_runtime::spawn_blocking(move || {
        let (host, port) = parse_host_port(&url);
        let addr = format!("{}:{}", host, port);
        let Ok(addr) = addr.parse() else {
            return None;
        };
        let start = Instant::now();
        let connect_timeout = Duration::from_millis(timeout_ms.min(5000));
        match TcpStream::connect_timeout(&addr, connect_timeout) {
            Ok(_) => {
                let ms = start.elapsed().as_millis() as u32;
                if ms > 800 {
                    None
                } else {
                    Some(ms)
                }
            }
            Err(_) => None,
        }
    })
    .await
    .ok()
    .flatten()
}

fn parse_host_port(url: &str) -> (String, u16) {
    let s = url.trim();
    let no_scheme = s.split_once("://").map(|(_, rest)| rest).unwrap_or(s);
    match no_scheme.rsplit_once(':') {
        Some((host, port)) => (host.to_string(), port.parse().unwrap_or(80)),
        None => (no_scheme.to_string(), 80),
    }
}

/// 创建不显示控制台的命令（Windows CREATE_NO_WINDOW）
fn silent_command(program: &str, args: &[&str]) -> std::process::Command {
    let mut cmd = std::process::Command::new(program);
    cmd.args(args);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd
}

/// Npcap 驱动检测（对应 ConnectionConnectGuard.hasNpcapDriver）。
#[tauri::command]
pub fn has_npcap_driver() -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::path::PathBuf;
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
        let candidates = [
            PathBuf::from(&windir).join("System32\\Npcap\\wpcap.dll"),
            PathBuf::from(&windir).join("SysWOW64\\Npcap\\wpcap.dll"),
            PathBuf::from(&windir).join("System32\\drivers\\npcap.sys"),
            PathBuf::from("C:\\Program Files\\Npcap\\NPFInstall.exe"),
            PathBuf::from("C:\\Program Files (x86)\\Npcap\\NPFInstall.exe"),
        ];
        if candidates.iter().any(|p| p.is_file()) {
            return true;
        }
        // 注册表检查
        for key in ["SOFTWARE\\Npcap", "SOFTWARE\\WOW6432Node\\Npcap"] {
            if let Ok(out) = silent_command("reg", &["query", &format!("HKLM\\{}", key)]).output() {
                if out.status.success() {
                    return true;
                }
            }
        }
        // 服务检查
        for svc in ["npcap", "npf"] {
            if let Ok(out) = silent_command("sc", &["qc", svc]).output() {
                let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
                if text.contains("npcap") {
                    return true;
                }
            }
        }
        false
    }
    #[cfg(not(target_os = "windows"))]
    {
        true
    }
}

/// 前端日志落盘（pushLog 镜像，写 %TEMP%\\astral-cmd.log）。
#[tauri::command]
pub fn frontend_log(msg: String) {
    simple::cmd_log(&format!("[fe] {}", msg));
}

/// 当前用户登录自启动。HKCU Run 不需要管理员权限，且启动参数可与
/// 「启动时最小化」同步更新；旧的 ONSTART/HIGHEST 计划任务会让普通用户设置失败。
#[tauri::command]
pub fn set_startup(enable: bool, minimize: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
        let output = if enable {
            let exe = std::env::current_exe()
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .to_string();
            let command = if minimize {
                format!("\"{exe}\" --autostart --minimized")
            } else {
                format!("\"{exe}\" --autostart")
            };
            silent_command(
                "reg",
                &["ADD", RUN_KEY, "/v", "Stellaris", "/t", "REG_SZ", "/d", &command, "/f"],
            )
            .output()
        } else {
            silent_command("reg", &["DELETE", RUN_KEY, "/v", "Stellaris", "/f"]).output()
        };
        match output {
            Ok(o) if o.status.success() => Ok(()),
            Ok(o) => {
                let message = String::from_utf8_lossy(&o.stderr).trim().to_string();
                Err(if message.is_empty() {
                    String::from_utf8_lossy(&o.stdout).trim().to_string()
                } else {
                    message
                })
            }
            Err(e) => Err(e.to_string()),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (enable, minimize);
        Ok(())
    }
}

/// 经被控端明确授权后，由 RemoteDesktopService 转发的白名单输入事件。
/// 该命令不接收任意脚本、进程启动或 shell 内容。
#[tauri::command]
pub fn remote_desktop_input(event: Value) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::*;
        use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SetCursorPos, SM_CXSCREEN, SM_CYSCREEN};
        let kind = event["kind"].as_str().unwrap_or_default();
        unsafe fn send_mouse(flags: MOUSE_EVENT_FLAGS, data: u32) -> Result<(), String> {
            let input = INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT { dx: 0, dy: 0, mouseData: data, dwFlags: flags, time: 0, dwExtraInfo: 0 },
                },
            };
            let sent = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
            if sent == 1 { Ok(()) } else { Err("系统拒绝了鼠标输入".into()) }
        }
        unsafe fn send_key(vk: u16, up: bool) -> Result<(), String> {
            let flags = if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) };
            let input = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT { wVk: VIRTUAL_KEY(vk), wScan: 0, dwFlags: flags, time: 0, dwExtraInfo: 0 },
                },
            };
            let sent = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
            if sent == 1 { Ok(()) } else { Err("系统拒绝了键盘输入".into()) }
        }
        match kind {
            "move" => {
                let x = event["x"].as_f64().unwrap_or(0.0).clamp(0.0, 1.0);
                let y = event["y"].as_f64().unwrap_or(0.0).clamp(0.0, 1.0);
                let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
                let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
                let px = (x * (width.saturating_sub(1)) as f64).round() as i32;
                let py = (y * (height.saturating_sub(1)) as f64).round() as i32;
                unsafe { SetCursorPos(px, py) }.map_err(|e| e.to_string())
            }
            "down" | "up" => {
                let down = kind == "down";
                let flags = match (event["button"].as_str().unwrap_or("left"), down) {
                    ("left", true) => MOUSEEVENTF_LEFTDOWN,
                    ("left", false) => MOUSEEVENTF_LEFTUP,
                    ("right", true) => MOUSEEVENTF_RIGHTDOWN,
                    ("right", false) => MOUSEEVENTF_RIGHTUP,
                    ("middle", true) => MOUSEEVENTF_MIDDLEDOWN,
                    ("middle", false) => MOUSEEVENTF_MIDDLEUP,
                    _ => return Err("鼠标按键无效".into()),
                };
                unsafe { send_mouse(flags, 0) }
            }
            "wheel" => {
                let delta = event["delta"].as_f64().unwrap_or(0.0).clamp(-1200.0, 1200.0) as i32;
                unsafe { send_mouse(MOUSEEVENTF_WHEEL, delta as u32) }
            }
            "keyDown" | "keyUp" => {
                let vk = event["vk"].as_u64().unwrap_or(0);
                if !(1..=254).contains(&vk) { return Err("虚拟键码无效".into()); }
                unsafe { send_key(vk as u16, kind == "keyUp") }
            }
            _ => Err("不支持的远程输入类型".into()),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = event;
        Err("当前平台暂不支持远程桌面输入".into())
    }
}

/// 为一起看创建独立网页窗口；先展示网站，再由前端请求系统画面共享选择器。
#[tauri::command]
pub fn open_watch_window(url: String, app: AppHandle) -> Result<String, String> {
    let parsed: tauri::Url = url.parse().map_err(|e| format!("网站地址无效: {e}"))?;
    let label = format!("watch-{}", uuid::Uuid::new_v4().simple());
    let builder = tauri::WebviewWindowBuilder::new(
        &app,
        &label,
        tauri::WebviewUrl::External(parsed),
    )
    .title("Stellaris 一起看")
    .inner_size(1280.0, 800.0);
    #[cfg(desktop)]
    let builder = builder.center();
    builder.build().map_err(|e| e.to_string())?;
    Ok(label)
}

#[tauri::command]
pub fn close_watch_window(label: String, app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(&label) {
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 外部链接打开兜底（优先使用 opener 插件）。
#[tauri::command]
pub fn set_desktop_lyrics_enabled(enabled: bool, app: AppHandle) -> Result<(), String> {
    const LABEL: &str = "desktop-lyrics";
    if !enabled {
        if let Some(window) = app.get_webview_window(LABEL) {
            window.close().map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    if app.get_webview_window(LABEL).is_some() {
        return Ok(());
    }
    let builder = tauri::WebviewWindowBuilder::new(
        &app,
        LABEL,
        tauri::WebviewUrl::App("index.html#desktopLyrics".into()),
    )
    .title("Stellaris 桌面歌词")
    .inner_size(880.0, 128.0)
    .min_inner_size(420.0, 86.0)
    .resizable(true);
    #[cfg(desktop)]
    let builder = builder
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .center();
    #[cfg(not(desktop))]
    return Err("桌面歌词仅支持桌面端".into());
    builder.build().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn set_desktop_lyrics_click_through(enabled: bool, app: AppHandle) -> Result<(), String> {
    #[cfg(desktop)]
    {
        if let Some(window) = app.get_webview_window("desktop-lyrics") {
            window
                .set_ignore_cursor_events(enabled)
                .map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    #[cfg(not(desktop))]
    {
        let _ = (enabled, app);
        Err("桌面歌词仅支持桌面端".into())
    }
}

#[tauri::command]
pub fn open_external(url: String) -> Result<(), String> {
    open_url_fallback(&url)
}

fn open_url_fallback(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        silent_command("cmd", &["/C", "start", "", url])
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

/* ==================== Android VPN ==================== */

/// 检查/申请系统 VPN 授权；返回 `{ granted }`。
#[cfg(target_os = "android")]
#[tauri::command]
pub fn vpn_prepare(app: AppHandle) -> Result<Value, String> {
    let status = app.vpnservice().prepare_vpn()?;
    serde_json::to_value(status).map_err(|e| e.to_string())
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub fn vpn_prepare(_app: AppHandle) -> Result<Value, String> {
    Err("VPN 服务仅支持 Android".to_string())
}

/// 启动 VpnService 并返回 TUN fd；返回 `{ fd }` 或 `{ errorMsg }`。
#[cfg(target_os = "android")]
#[tauri::command]
pub fn vpn_start(app: AppHandle, options: Value) -> Result<Value, String> {
    let req: StartVpnRequest = serde_json::from_value(options).map_err(|e| e.to_string())?;
    let status = app.vpnservice().start_vpn(req)?;
    serde_json::to_value(status).map_err(|e| e.to_string())
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub fn vpn_start(_app: AppHandle, _options: Value) -> Result<Value, String> {
    Err("VPN 服务仅支持 Android".to_string())
}

/// 关闭 VpnService 并回收 TUN。
#[cfg(target_os = "android")]
#[tauri::command]
pub fn vpn_stop(app: AppHandle) -> Result<(), String> {
    app.vpnservice().stop_vpn()?;
    Ok(())
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub fn vpn_stop(_app: AppHandle) -> Result<(), String> {
    Ok(())
}

/* ==================== UDP 日志桥 ==================== */

/// 启动 UDP 127.0.0.1:9999 监听，把 Rust 核心推送的中文事件文本
/// 转发为 webview 的 `rust-log` 事件（对应 Flutter 端 LogCapture）。
pub fn start_log_bridge(app: AppHandle) {
    std::thread::spawn(move || {
        use std::net::UdpSocket;
        let socket = match UdpSocket::bind("127.0.0.1:9999") {
            Ok(s) => s,
            Err(e) => {
                eprintln!("日志 UDP 桥绑定失败: {}", e);
                return;
            }
        };
        let mut buf = [0u8; 4096];
        loop {
            match socket.recv_from(&mut buf) {
                Ok((len, _)) => {
                    let text = String::from_utf8_lossy(&buf[..len]).to_string();
                    let _ = app.emit("rust-log", text.clone());
                }
                Err(e) => {
                    eprintln!("日志 UDP 桥接收失败: {}", e);
                }
            }
        }
    });
}
