//! Astral Tauri 命令层 —— 把原 Flutter 版 rust/ 网络核心封装为 Tauri 2 命令。
//! 命令名与前端 `src/lib/rust/api.js` 的 invoke 调用一一对应；
//! 返回数据形状与浏览器 Mock 模式完全一致（camelCase）。

use astral_core::api::serde_types::*;
use astral_core::api::{firewall, hops, magic_wall, nat_test, simple};
use serde_json::Value;
use std::net::TcpStream;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/* ==================== EasyTier 核心 ==================== */

#[tauri::command]
pub fn easytier_version() -> Result<String, String> {
    simple::easytier_version()
}

#[tauri::command]
pub async fn get_running_info() -> String {
    simple::get_running_info().await
}

#[tauri::command]
pub async fn get_network_status() -> Result<Value, String> {
    let status = simple::get_network_status().await;
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

/// 创建虚拟网络（fire-and-forget，与原版 JoinHandle 语义一致：
/// 成败由前端轮询 get_running_info 判定）。
#[tauri::command]
pub fn create_server(config: CreateServerConfig) -> Result<(), String> {
    let flag = astral_core::api::simple::FlagsC {
        default_protocol: config.flag.default_protocol,
        dev_name: config.flag.dev_name,
        enable_encryption: config.flag.enable_encryption,
        enable_ipv6: config.flag.enable_ipv6,
        mtu: config.flag.mtu,
        latency_first: config.flag.latency_first,
        enable_exit_node: config.flag.enable_exit_node,
        no_tun: config.flag.no_tun,
        use_smoltcp: config.flag.use_smoltcp,
        relay_network_whitelist: config.flag.relay_network_whitelist,
        disable_p2p: config.flag.disable_p2p,
        enable_udp_broadcast_relay: config.flag.enable_udp_broadcast_relay,
        relay_all_peer_rpc: config.flag.relay_all_peer_rpc,
        disable_udp_hole_punching: config.flag.disable_udp_hole_punching,
        disable_tcp_hole_punching: config.flag.disable_tcp_hole_punching,
        multi_thread: config.flag.multi_thread,
        data_compress_algo: config.flag.data_compress_algo,
        bind_device: config.flag.bind_device,
        enable_kcp_proxy: config.flag.enable_kcp_proxy,
        disable_kcp_input: config.flag.disable_kcp_input,
        disable_relay_kcp: config.flag.disable_relay_kcp,
        proxy_forward_by_system: config.flag.proxy_forward_by_system,
        accept_dns: config.flag.accept_dns,
        private_mode: config.flag.private_mode,
        enable_quic_proxy: config.flag.enable_quic_proxy,
        disable_quic_input: config.flag.disable_quic_input,
        disable_sym_hole_punching: config.flag.disable_sym_hole_punching,
        tcp_whitelist: config.flag.tcp_whitelist,
        udp_whitelist: config.flag.udp_whitelist,
        socks5_port: config.flag.socks5_port,
    };
    // forwards 恒为空（与原版 ServerConfigBuilder.build() 一致）
    let _ = simple::create_server(
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
    );
    Ok(())
}

#[tauri::command]
pub fn close_server() {
    simple::close_server();
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
    tokio::task::spawn_blocking(move || {
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

/// 开机自启动（对应 startup_url_scheme.dart：Windows 计划任务方式）。
#[tauri::command]
pub fn set_startup(enable: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let exe = std::env::current_exe()
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .to_string();
        let task_arg = format!("\"{}\" --autostart", exe);
        let output = if enable {
            silent_command(
                "schtasks",
                &[
                    "/create",
                    "/tn",
                    "Astral",
                    "/tr",
                    &task_arg,
                    "/sc",
                    "ONSTART",
                    "/rl",
                    "HIGHEST",
                    "/f",
                ],
            )
            .output()
        } else {
            silent_command("schtasks", &["/delete", "/tn", "Astral", "/f"]).output()
        };
        match output {
            Ok(o) if o.status.success() => Ok(()),
            Ok(o) => Err(String::from_utf8_lossy(&o.stderr).to_string()),
            Err(e) => Err(e.to_string()),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = enable;
        Ok(())
    }
}

/// 外部链接打开兜底（优先使用 opener 插件）。
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
