/* ---- 安全 stdout 打印：GUI 子系统（windows_subsystem="windows"）下 ----
 * stdout 句柄无效，print!/println! 写失败会 panic：
 *   · 命令线程 panic  => Tauri IPC 响应丢失，前端 invoke 永远挂起
 *     （"点连接无反应"的根因：create_server 入口的 print!）
 *   · 事件桥任务 panic => Rust 日志转发整体中断
 * 以下宏忽略写失败，替代全部 stdout/stderr 打印。 */
macro_rules! safe_print {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        let _ = std::io::stdout().lock().write_all(format!($($arg)*).as_bytes());
    }};
}
macro_rules! safe_println {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        let _ = std::io::stdout().lock().write_all(format!($($arg)*).as_bytes());
        let _ = std::io::stdout().lock().write_all(b"\n");
    }};
}
macro_rules! safe_eprintln {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        let _ = std::io::stderr().lock().write_all(format!($($arg)*).as_bytes());
        let _ = std::io::stderr().lock().write_all(b"\n");
    }};
}
pub use easytier::common::config::ConfigFileControl;
use easytier::common::config::{ConfigLoader, NetworkIdentity, PeerConfig, TomlConfigLoader};
#[cfg(any(target_os = "android", target_os = "ios"))]
use easytier::instance::factory::attach_mobile_tun_fd;
pub use easytier::{
    common::global_ctx::{EventBusSubscriber, GlobalCtxEvent},
    instance::factory::{
        native_instance_manager_with_runtime, subscribe_native_instance_event, NativeCoreInstance,
        NativeInstanceManager,
    },
    proto,
    proto::{
        api::instance::{PeerRoutePair, Route},
        api::manage::NetworkInstanceRunningInfo,
        common::NatType,
    },
};
use easytier_core::instance::CoreInstanceState;
use easytier_core::management::network_instance_running_info;
use lazy_static::lazy_static;
use serde_json::json;
pub use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::RwLock;
pub use tokio::task::JoinHandle;

pub static DEFAULT_ET_DNS_ZONE: &str = "as.net.";

/* ---- EasyTier 2.7 适配 ----
 * 2.7 重构了实例层：launcher::NetworkInstance / get_running_info /
 * get_tun_fd_sender 全部移除，改为 InstanceManager<NativeInstanceFactory>
 * + CoreInstance（management 体系）。本层保持对外 API（FlagsC /
 * create_server / close_server / set_tun_fd / get_running_info /
 * get_network_status ...）签名不变，commands.rs 与前端契约零改动：
 *   · MANAGER   进程级单例 InstanceManager（绑定本层 tokio RT）
 *   · INSTANCE_ID 当前网络实例的 Uuid（同一时刻至多一个）
 *   · 状态装配走官方 network_instance_running_info(instance) —— 与
 *     旧 get_running_info 同构（dev_name/my_node_info/routes/peers/
 *     peer_route_pairs），前端 JSON 契约原样保留。
 *   · 配置构建仍走 TomlConfigLoader（2.7 的 ConfigLoader trait setter
 *     全部保留），dump -> TomlConfig::new_from_str 喂给 manager。 */
lazy_static! {
    static ref RT: Runtime = Runtime::new().expect("创建 Tokio 运行时失败");
    static ref MANAGER: NativeInstanceManager =
        native_instance_manager_with_runtime(RT.handle().clone());
    static ref INSTANCE_ID: RwLock<Option<uuid::Uuid>> = RwLock::new(None);
}

fn peer_conn_info_to_string(p: proto::api::instance::PeerConnInfo) -> String {
    format!(
        "my_peer_id: {}, dst_peer_id: {}, tunnel_info: {:?}",
        p.my_peer_id, p.peer_id, p.tunnel
    )
}

/// 命令/任务文件日志：追加 %TEMP%\\astral-cmd.log（不依赖 UDP 桥与前端），
/// 定位 create_server 全链路（命令线程 / RT 任务）死点；unwrap panic 也会在此留痕前记下。
pub fn cmd_log(msg: &str) {
    use std::io::Write;
    let path = std::env::temp_dir().join("astral-cmd.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "[{:?}] {}", std::time::SystemTime::now(), msg);
    }
}

/// 启动时预热 tokio 运行时：close_server/create_server 首次访问 RT 的
/// lazy_static 初始化曾卡死命令线程（两次进入、永不返回）。
/// 在 setup 阶段（非命令线程）提前完成初始化。
pub fn preinit_runtime() {
    cmd_log("preinit_runtime: 开始初始化 tokio 运行时");
    let handle = RT.handle().clone();
    cmd_log("preinit_runtime: 运行时就绪");
    let _ = handle.spawn(async {
        cmd_log("preinit_runtime: 测试任务执行");
    });
}

pub fn send_udp_to_localhost(message: &str) -> Result<(), String> {
    use std::net::UdpSocket;

    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(e) => return Err(format!("绑定UDP套接字失败: {}", e)),
    };

    match socket.send_to(message.as_bytes(), "127.0.0.1:9999") {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("发送UDP数据失败: {}", e)),
    }
}

pub fn handle_event(mut events: EventBusSubscriber) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            match events.recv().await {
                Ok(e) => {
                    //  safe_println!("Received event: {:?}", e);
                    match e {
                        GlobalCtxEvent::PeerAdded(p) => {
                            safe_println!("{}", format!("新节点已添加。节点ID: {}", p));
                            let _ = send_udp_to_localhost(&format!("新节点已添加。节点ID: {}", p));
                        }
                        GlobalCtxEvent::PeerRemoved(p) => {
                            safe_println!("{}", format!("节点已移除。节点ID: {}", p));
                            let _ = send_udp_to_localhost(&format!("节点已移除。节点ID: {}", p));
                        }
                        GlobalCtxEvent::PeerConnAdded(p) => {
                            let conn_info = peer_conn_info_to_string(p);
                            let msg = format!("新节点连接已添加。连接信息: {}", conn_info);
                            safe_println!("{}", msg);
                            let _ = send_udp_to_localhost(&msg);
                        }
                        GlobalCtxEvent::PeerConnRemoved(p) => {
                            let msg = format!(
                                "节点连接已移除。连接信息: {}",
                                peer_conn_info_to_string(p)
                            );
                            safe_println!("{}", msg);
                            let _ = send_udp_to_localhost(&msg);
                        }
                        GlobalCtxEvent::ListenerAddFailed(p, msg) => {
                            let msg = format!("监听器添加失败。监听器: {}, 消息: {}", p, msg);
                            safe_println!("{}", msg);
                            let _ = send_udp_to_localhost(&msg);
                        }
                        GlobalCtxEvent::ListenerAcceptFailed(p, msg) => {
                            let msg = format!("监听器接受失败。监听器: {}, 消息: {}", p, msg);
                            safe_println!("{}", msg);
                            let _ = send_udp_to_localhost(&msg);
                        }
                        GlobalCtxEvent::ListenerAdded(p) => {
                            if p.scheme() == "ring" {
                                continue;
                            }
                            let msg = format!("新监听器已添加。监听器: {}", p);
                            safe_println!("{}", msg);
                            let _ = send_udp_to_localhost(&msg);
                        }
                        GlobalCtxEvent::ConnectionAccepted(local, remote) => {
                            let msg = format!("新连接已接受。本地: {}, 远程: {}", local, remote);
                            safe_println!("{}", msg);
                            let _ = send_udp_to_localhost(&msg);
                        }
                        GlobalCtxEvent::ConnectionError(local, remote, err) => {
                            let msg = format!(
                                "连接错误。本地: {}, 远程: {}, 错误: {}",
                                local, remote, err
                            );
                            safe_println!("{}", msg);
                            let _ = send_udp_to_localhost(&msg);
                        }
                        GlobalCtxEvent::TunDeviceReady(dev) => {
                            let msg = format!("TUN 设备就绪。设备: {}", dev);
                            safe_println!("{}", msg);
                            let _ = send_udp_to_localhost(&msg);
                        }
                        GlobalCtxEvent::TunDeviceError(err) => {
                            let msg = format!("TUN 设备错误。错误: {}", err);
                            safe_println!("{}", msg);
                            let _ = send_udp_to_localhost(&msg);
                        }
                        GlobalCtxEvent::Connecting(dst) => {
                            let msg = format!("正在连接到节点。目标: {}", dst);
                            safe_println!("{}", msg);
                            let _ = send_udp_to_localhost(&msg);
                        }
                        GlobalCtxEvent::ConnectError(dst, ip_version, err) => {
                            let msg = format!(
                                "连接到节点错误。目标: {}, IP版本: {}, 错误: {}",
                                dst, ip_version, err
                            );
                            safe_println!("{}", msg);
                            let _ = send_udp_to_localhost(&msg);
                        }
                        GlobalCtxEvent::VpnPortalStarted(portal) => {
                            let msg = format!("VPN 门户已启动。门户: {}", portal);
                            safe_println!("{}", msg);
                            let _ = send_udp_to_localhost(&msg);
                        }
                        GlobalCtxEvent::VpnPortalClientConnected(portal, client_addr) => {
                            let msg = format!(
                                "VPN 门户客户端已连接。门户: {}, 客户端地址: {}",
                                portal, client_addr
                            );
                            safe_println!("{}", msg);
                            let _ = send_udp_to_localhost(&msg);
                        }
                        GlobalCtxEvent::VpnPortalClientDisconnected(portal, client_addr) => {
                            let msg = format!(
                                "VPN 门户客户端已断开连接。门户: {}, 客户端地址: {}",
                                portal, client_addr
                            );
                            safe_println!("{}", msg);
                            let _ = send_udp_to_localhost(&msg);
                        }
                        GlobalCtxEvent::DhcpIpv4Changed(old, new) => {
                            let msg = format!("DHCP IP 已更改。旧: {:?}, 新: {:?}", old, new);
                            safe_println!("{}", msg);
                            let _ = send_udp_to_localhost(&msg);
                        }
                        GlobalCtxEvent::DhcpIpv4Conflicted(ip) => {
                            let msg = format!("DHCP IP 冲突。IP: {:?}", ip);
                            safe_println!("{}", msg);
                            let _ = send_udp_to_localhost(&msg);
                        }
                        GlobalCtxEvent::PortForwardAdded(port_forward_config_pb) => {
                            let msg = format!("端口转发已添加。配置: {:?}", port_forward_config_pb);
                            safe_println!("{}", msg);
                            let _ = send_udp_to_localhost(&msg);
                        }
                        _ => {}
                    }
                }
                Err(err) => {
                    safe_eprintln!("接收事件错误: {:?}", err);
                    // 根据错误类型决定是否中断循环
                    match err {
                        tokio::sync::broadcast::error::RecvError::Closed => {
                            let msg = "事件通道已关闭，停止事件处理。";
                            safe_println!("{}", msg);
                            let _ = send_udp_to_localhost(msg);
                            break; // Exit the loop if the channel is closed
                        }
                        tokio::sync::broadcast::error::RecvError::Lagged(n) => {
                            let msg = format!("事件处理滞后，丢失了 {} 个事件。", n);
                            safe_eprintln!("{}", msg);
                            let _ = send_udp_to_localhost(&msg);
                            // Decide if lagging is critical enough to break or just log
                        }
                    }
                }
            }
        }
    })
}

/// 当前实例（存在且未停止）。
async fn current_instance() -> Option<Arc<NativeCoreInstance>> {
    let id = *INSTANCE_ID.read().await;
    let id = id?;
    let instance = MANAGER.instance(id)?;
    if instance.state() == CoreInstanceState::Stopped {
        return None;
    }
    Some(instance)
}

/// 装配当前实例的运行快照（官方 management 层实现，与 2.6 时代
/// NetworkInstance::get_running_info 同构）。
/// 2.7 的 peer_route_pairs 不保证含本机节点（旧版含）—— 拓扑图以本机
/// 为根、成员列表固定显示本机卡，缺失则在此统一注入，所有消费方生效。
async fn fetch_running_info() -> Option<NetworkInstanceRunningInfo> {
    let instance = current_instance().await?;
    let mut info = network_instance_running_info(instance.as_ref()).await.ok()?;
    if let Some(my) = info.my_node_info.as_ref() {
        let my_peer_id = my.peer_id;
        if !info
            .peer_route_pairs
            .iter()
            .any(|p| p.route.as_ref().map_or(false, |r| r.peer_id == my_peer_id))
        {
            let mut local_route = Route::default();
            local_route.peer_id = my.peer_id;
            local_route.ipv4_addr = my.virtual_ipv4.clone();
            local_route.next_hop_peer_id = my.peer_id;
            local_route.cost = 0;
            local_route.path_latency = 0;
            local_route.hostname = my.hostname.clone();
            local_route.inst_id = "local".to_string();
            local_route.version = my.version.clone();
            local_route.stun_info = my.stun_info.clone();
            let local_peer = info
                .peers
                .iter()
                .find(|p| p.peer_id == my_peer_id)
                .cloned();
            info.peer_route_pairs.insert(
                0,
                PeerRoutePair {
                    route: Some(local_route),
                    peer: local_peer,
                },
            );
        }
    }
    Some(info)
}

// 返回EasyTier的版本号
pub fn easytier_version() -> Result<String, String> {
    Ok(easytier::VERSION.to_string())
}

// 是否在运行
/// 同步包装：在任意非 tokio 线程上以 RT 驱动 is_easytier_running。
pub fn rt_block_on_is_running() -> bool {
    RT.block_on(async { is_easytier_running().await })
}

pub async fn is_easytier_running() -> bool {
    current_instance().await.is_some()
}

// 定义节点跳跃统计信息结构体
pub struct NodeHopStats {
    pub peer_id: u32,
    pub target_ip: String, // 目标节点IP
    pub latency_ms: f64,   // 延迟(毫秒)
    pub packet_loss: f32,  // 丢包率
    pub node_name: String, // 节点名称
}

// 定义节点连接统计信息结构体
pub struct KVNodeConnectionStats {
    pub conn_type: String, // 连接类型
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
}
// 定义节点信息结构体
pub struct KVNodeInfo {
    pub peer_id: u32,
    pub hostname: String,
    pub ipv4: String,
    pub latency_ms: f64,
    pub nat: String, // NAT类型
    // NodeHopStats 列表 从近到远
    pub hops: Vec<NodeHopStats>,
    pub loss_rate: f32,
    pub connections: Vec<KVNodeConnectionStats>,

    pub tunnel_proto: String,
    pub conn_type: String, // 连接类型
    pub rx_bytes: u64,
    pub tx_bytes: u64,

    pub version: String,
    pub cost: i32,
    pub proxy_cidrs: Vec<String>,
}
// 定义节点网络状态结构体
pub struct KVNetworkStatus {
    pub total_nodes: usize,
    pub nodes: Vec<KVNodeInfo>,
}

// 获取网络中所有节点的IP地址列表
pub async fn get_ips() -> Vec<String> {
    let mut result = Vec::new();
    let info = fetch_running_info().await;
    if let Some(info) = info {
        for route in &info.routes {
            if let Some(ipv4_addr) = &route.ipv4_addr {
                if let Some(addr) = &ipv4_addr.address {
                    let ip = format!(
                        "{}.{}.{}.{}/{}",
                        (addr.addr >> 24) & 0xFF,
                        (addr.addr >> 16) & 0xFF,
                        (addr.addr >> 8) & 0xFF,
                        addr.addr & 0xFF,
                        ipv4_addr.network_length
                    );
                    // Avoid duplicates
                    if !result.contains(&ip) {
                        result.push(ip);
                    }
                }
            }
        }
    }
    result
}

// 设置TUN设备的文件描述符
pub async fn set_tun_fd(fd: i32) -> Result<(), String> {
    let instance = current_instance()
        .await
        .ok_or_else(|| "No instance available".to_string())?;
    let instance_id = instance.instance_id();

    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        let _ = instance_id;
        attach_mobile_tun_fd(&MANAGER, fd, None)
            .await
            .map(|_| ())
            .map_err(|e| format!("failed to attach tun fd: {e:#}"))
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let _ = instance;
        MANAGER
            .attach_tun_fd(instance_id, fd)
            .map_err(|e| format!("failed to attach tun fd: {e:#}"))
    }
}

pub async fn get_running_info() -> String {
    let Some(info) = fetch_running_info().await else {
        return "null".to_string();
    };
    // 前端契约：my_node_info.virtual_ipv4.address.addr（u32 主机序整数）
    let virtual_ipv4 = info
        .my_node_info
        .as_ref()
        .and_then(|node| node.virtual_ipv4.as_ref())
        .map(|inet| {
            json!({
                "address": inet.address.as_ref().map(|a| json!({ "addr": a.addr })),
                "network_length": inet.network_length,
            })
        })
        .unwrap_or(serde_json::Value::Null);
    serde_json::to_string(&json!({
        "dev_name": info.dev_name,
        "my_node_info": {
            "virtual_ipv4": virtual_ipv4,
        },
        "routes": info.routes,
        "peer_route_pairs": info.peer_route_pairs,
    }))
    .unwrap_or_else(|_| "null".to_string())
}

pub struct FlagsC {
    pub default_protocol: String,
    pub dev_name: String,
    pub enable_encryption: bool,
    pub enable_ipv6: bool,
    pub mtu: u32,
    pub latency_first: bool,
    pub enable_exit_node: bool,
    pub no_tun: bool,
    pub use_smoltcp: bool,
    pub relay_network_whitelist: String,
    pub disable_p2p: bool,
    pub enable_udp_broadcast_relay: bool,
    pub relay_all_peer_rpc: bool,
    pub disable_udp_hole_punching: bool,
    pub disable_tcp_hole_punching: bool,
    pub multi_thread: bool,
    pub data_compress_algo: i32,
    pub bind_device: bool,
    pub enable_kcp_proxy: bool,
    pub disable_kcp_input: bool,
    pub disable_relay_kcp: bool,
    pub proxy_forward_by_system: bool,
    pub accept_dns: bool,
    pub private_mode: bool,
    pub enable_quic_proxy: bool,
    pub disable_quic_input: bool,
    pub disable_sym_hole_punching: bool,
    pub tcp_whitelist: String,
    pub udp_whitelist: String,
    /// SOCKS5 监听端口，0 表示禁用
    pub socks5_port: u16,
}

pub struct Forward {
    pub bind_addr: String,
    pub dst_addr: String,
    pub proto: String,
}

// 创建服务器
pub fn create_server(
    username: String,
    enable_dhcp: bool,
    specified_ip: String,
    room_name: String,
    room_password: String,
    severurl: Vec<String>,
    onurl: Vec<String>,
    cidrs: Vec<String>,
    forwards: Vec<Forward>,
    flag: FlagsC,
) -> JoinHandle<Result<(), String>> {
    safe_print!("{}", format!("创建服务器: {}，启用DHCP: {}, 指定IP: {}, 房间名称: {}, 房间密码: {}, 服务器URL: {:?}, 监听器URL: {:?}", username, enable_dhcp, specified_ip, room_name, room_password, severurl, onurl));
    RT.spawn(async move {
        // Create config with better error handling
        cmd_log("task: create_server 任务开始，构建配置");
        let cfg = TomlConfigLoader::default();

        // Set listeners with proper error handling
        let mut listeners = Vec::new();
        for url in onurl {
            match url.parse() {
                Ok(parsed) => listeners.push(parsed),
                Err(e) => return Err(format!("Invalid listener URL: {}, error: {}", url, e)),
            }
        }
        cfg.set_listeners(listeners);

        // Set hostname and other settings
        cfg.set_hostname(Some(username));
        cfg.set_dhcp(enable_dhcp);
        for c in cidrs {
            let _ = cfg.add_proxy_cidr(c.parse().unwrap(), None);
        }
        // forwards 恒为空（与原版 ServerConfigBuilder.build() 一致）；
        // 2.7 中端口转发属于网关运行时配置，单实例场景不使用。
        let _ = forwards;

        // Set flags more efficiently by directly mapping from input
        let mut flags = cfg.get_flags();
        flags.default_protocol = flag.default_protocol;
        flags.dev_name = "astral".to_string();
        flags.enable_encryption = flag.enable_encryption;
        flags.enable_ipv6 = flag.enable_ipv6;
        flags.latency_first = flag.latency_first;
        flags.enable_exit_node = flag.enable_exit_node;
        flags.no_tun = flag.no_tun;
        flags.use_smoltcp = flag.use_smoltcp;
        flags.relay_network_whitelist = flag.relay_network_whitelist;
        flags.disable_p2p = flag.disable_p2p;
        flags.enable_udp_broadcast_relay = flag.enable_udp_broadcast_relay;
        flags.relay_all_peer_rpc = flag.relay_all_peer_rpc;
        flags.disable_udp_hole_punching = flag.disable_udp_hole_punching;
        flags.disable_tcp_hole_punching = flag.disable_tcp_hole_punching;
        flags.multi_thread = flag.multi_thread;
        flags.data_compress_algo = flag.data_compress_algo;
        flags.bind_device = flag.bind_device;
        flags.enable_kcp_proxy = flag.enable_kcp_proxy;
        flags.disable_kcp_input = flag.disable_kcp_input;
        flags.disable_relay_kcp = flag.disable_relay_kcp;
        flags.proxy_forward_by_system = flag.proxy_forward_by_system;
        flags.accept_dns = flag.accept_dns;
        flags.mtu = flag.mtu;

        flags.private_mode = flag.private_mode;
        flags.enable_quic_proxy = flag.enable_quic_proxy;
        flags.disable_quic_input = flag.disable_quic_input;
        flags.disable_sym_hole_punching = flag.disable_sym_hole_punching;
        cfg.set_flags(flags);

        if flag.socks5_port > 0 {
            let portal = format!("socks5://127.0.0.1:{}", flag.socks5_port);
            match portal.parse() {
                Ok(url) => cfg.set_socks5_portal(Some(url)),
                Err(e) => return Err(format!("Invalid SOCKS5 portal: {}, error: {}", portal, e)),
            }
        }

        // Set port whitelists
        if !flag.tcp_whitelist.is_empty() {
            let tcp_ports: Vec<String> = flag
                .tcp_whitelist
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            cfg.set_tcp_whitelist(tcp_ports);
        }
        if !flag.udp_whitelist.is_empty() {
            let udp_ports: Vec<String> = flag
                .udp_whitelist
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            cfg.set_udp_whitelist(udp_ports);
        }

        // Configure peer connections with proper error handling
        let mut peer_configs = Vec::new();
        for url in severurl {
            match url.parse() {
                Ok(uri) => peer_configs.push(PeerConfig {
                    uri,
                    peer_public_key: None,
                }),
                Err(e) => return Err(format!("Invalid server URL: {}, error: {}", url, e)),
            }
        }
        cfg.set_peers(peer_configs);

        // Set IP if DHCP is disabled
        if !enable_dhcp && !specified_ip.is_empty() {
            let ip_str = format!("{}/24", specified_ip);
            match ip_str.parse() {
                Ok(ip) => cfg.set_ipv4(Some(ip)),
                Err(e) => {
                    return Err(format!(
                        "Invalid IP address: {}, error: {}",
                        specified_ip, e
                    ))
                }
            }
        }

        // Set network identity
        cfg.set_network_identity(NetworkIdentity::new(room_name, room_password));

        // TomlConfigLoader -> TomlConfig（官方 INI：dump 后重新解析，
        // 两条路径均为公开稳定 API）
        let dumped = cfg.dump();
        let toml_config = easytier_core::config::toml::TomlConfig::new_from_str(&dumped)
            .map_err(|e| format!("配置解析失败: {e:#}"))?;

        cmd_log("task: 提交 InstanceManager.run_network_instance");
        let instance_id = MANAGER
            .run_network_instance(toml_config, ConfigFileControl::STATIC_CONFIG)
            .map_err(|e| {
                cmd_log(&format!("task: run_network_instance 失败: {e:#}"));
                format!("启动网络实例失败: {e:#}")
            })?;
        cmd_log(&format!("task: 实例已创建 id={instance_id}"));
        *INSTANCE_ID.write().await = Some(instance_id);

        // 订阅事件（与旧 network.start() 返回的订阅等价）
        if let Some(instance) = MANAGER.instance(instance_id) {
            if let Some(events) = subscribe_native_instance_event(&instance) {
                handle_event(events);
            }
        }
        safe_println!("instance {} started", instance_id);
        cmd_log("task: 实例已启动并写入 INSTANCE_ID");

        Ok(())
    })
}

// 关闭服务器实例
pub fn close_server() {
    cmd_log("close_server: 步骤1-进入");
    cmd_log("close_server: 步骤2-准备 RT.spawn");
    RT.spawn(async {
        cmd_log("close_server: 步骤3-任务已提交到运行时");
        let id = INSTANCE_ID.write().await.take();
        match id {
            Some(instance_id) => {
                safe_println!("正在关闭实例 {}", instance_id);
                if let Err(e) = MANAGER.delete_network_instances(vec![instance_id]).await {
                    safe_eprintln!("关闭实例失败: {e:#}");
                    cmd_log(&format!("close_server: delete 失败: {e:#}"));
                } else {
                    safe_println!("实例已成功关闭");
                    cmd_log("close_server: 实例已删除并停止");
                }
            }
            None => {
                safe_println!("没有找到需要关闭的实例");
                cmd_log("close_server: 无实例可关");
            }
        }
    });
}

// 获取节点路由对（本机兜底注入已在 fetch_running_info 统一处理）
pub async fn get_peer_route_pairs() -> Result<Vec<PeerRoutePair>, String> {
    let info = fetch_running_info()
        .await
        .ok_or_else(|| "没有运行中的网络实例".to_string())?;
    Ok(info.peer_route_pairs)
}

// 获取网络状态信息
pub async fn get_network_status() -> KVNetworkStatus {
    let info = match fetch_running_info().await {
        Some(info) => info,
        None => {
            return KVNetworkStatus {
                total_nodes: 0,
                nodes: Vec::new(),
            }
        }
    };

    let pairs = info.peer_route_pairs.clone();
    let my_node_info = info.my_node_info.clone();
    let local_peer_id = my_node_info.as_ref().map_or(0, |n| n.peer_id);

    let mut nodes = Vec::new();
    for pair in pairs.iter() {
        if let Some(route) = &pair.route {
            let cost = route.cost;
            // 使用 get_running_info 中的逻辑来获取IPv4地址
            let ipv4 = route
                .ipv4_addr
                .as_ref()
                .and_then(|addr| addr.address.as_ref())
                .map(|a| {
                    format!(
                        "{}.{}.{}.{}",
                        (a.addr >> 24) & 0xFF,
                        (a.addr >> 16) & 0xFF,
                        (a.addr >> 8) & 0xFF,
                        a.addr & 0xFF
                    )
                })
                .unwrap_or_else(|| "0.0.0.0".to_string());
            let mut node_info = KVNodeInfo {
                peer_id: route.peer_id,
                hostname: route.hostname.clone(),

                hops: {
                    // 新建递归函数收集完整路径
                    fn collect_hops(
                        pairs: &[PeerRoutePair],
                        current_peer_id: u32,
                        mut path: Vec<NodeHopStats>,
                        visited: &mut std::collections::HashSet<u32>,
                    ) -> Vec<NodeHopStats> {
                        if visited.contains(&current_peer_id) {
                            return path;
                        }
                        visited.insert(current_peer_id);

                        // 查找当前节点的信息
                        if let Some(pair) = pairs.iter().find(|p| {
                            p.route
                                .as_ref()
                                .map_or(false, |r| r.peer_id == current_peer_id)
                        }) {
                            if let Some(route) = &pair.route {
                                // 获取IP地址
                                let ip = route
                                    .ipv4_addr
                                    .as_ref()
                                    .and_then(|addr| addr.address.as_ref())
                                    .map(|a| {
                                        format!(
                                            "{}.{}.{}.{}",
                                            (a.addr >> 24) & 0xFF,
                                            (a.addr >> 16) & 0xFF,
                                            (a.addr >> 8) & 0xFF,
                                            a.addr & 0xFF
                                        )
                                    })
                                    .unwrap_or_default();

                                // 计算延迟和丢包率
                                let (latency, loss) = pair.peer.as_ref().map_or((0.0, 0.0), |p| {
                                    let min_latency = p
                                        .conns
                                        .iter()
                                        .filter_map(|c| c.stats.as_ref().map(|s| s.latency_us))
                                        .min()
                                        .unwrap_or(0)
                                        as f64
                                        / 1000.0;

                                    let avg_loss = p.conns.iter().map(|c| c.loss_rate).sum::<f32>()
                                        / p.conns.len().max(1) as f32;

                                    (min_latency, avg_loss as f64)
                                });

                                // 添加当前节点到路径
                                path.push(NodeHopStats {
                                    peer_id: current_peer_id,
                                    target_ip: ip,
                                    latency_ms: latency,
                                    packet_loss: loss as f32,
                                    node_name: route.hostname.clone(),
                                });

                                // 如果下一跳不是自己，继续递归
                                if route.next_hop_peer_id != current_peer_id
                                    && route.next_hop_peer_id != 0
                                {
                                    // 查找下一跳节点
                                    return collect_hops(
                                        pairs,
                                        route.next_hop_peer_id,
                                        path,
                                        visited,
                                    );
                                }
                            }
                        }
                        path
                    }

                    // 使用路由表中的next_hop_peer_id构建完整路径
                    let mut hops = Vec::new();
                    if let Some(route) = &pair.route {
                        let mut visited = std::collections::HashSet::new();
                        let target_ip = route
                            .ipv4_addr
                            .as_ref()
                            .and_then(|addr| addr.address.as_ref())
                            .map(|a| {
                                format!(
                                    "{}.{}.{}.{}",
                                    (a.addr >> 24) & 0xFF,
                                    (a.addr >> 16) & 0xFF,
                                    (a.addr >> 8) & 0xFF,
                                    a.addr & 0xFF
                                )
                            })
                            .unwrap_or_default();

                        // 从当前节点开始，收集到目标节点的完整路径
                        // 先添加本地节点信息
                        if let Some(local_node) = &my_node_info {
                            // 添加本地节点作为起点
                            hops.push(NodeHopStats {
                                peer_id: local_peer_id,
                                target_ip: local_node
                                    .virtual_ipv4
                                    .as_ref()
                                    .and_then(|addr| addr.address.as_ref())
                                    .map(|a| {
                                        format!(
                                            "{}.{}.{}.{}",
                                            (a.addr >> 24) & 0xFF,
                                            (a.addr >> 16) & 0xFF,
                                            (a.addr >> 8) & 0xFF,
                                            a.addr & 0xFF
                                        )
                                    })
                                    .unwrap_or_else(|| local_node.hostname.clone()),
                                latency_ms: 0.0,
                                packet_loss: 0.0,
                                node_name: local_node.hostname.clone(),
                            });

                            // 查找从本地到目标节点的路由
                            if let Some(local_route) =
                                info.routes.iter().find(|r| r.peer_id == route.peer_id)
                            {
                                // 收集中间节点
                                let mut next_hops = collect_hops(
                                    pairs.as_slice(),
                                    local_route.next_hop_peer_id,
                                    Vec::new(),
                                    &mut visited,
                                );
                                hops.append(&mut next_hops);

                                // 确保目标节点被添加到路径中
                                // 检查最后一个节点是否是目标节点
                                let last_node_is_target = hops.last().map_or(false, |last| {
                                    // 比较目标IP
                                    last.target_ip == target_ip
                                });

                                // 如果最后一个节点不是目标节点，则添加目标节点
                                if !last_node_is_target && !visited.contains(&route.peer_id) {
                                    let (latency, loss) =
                                        pair.peer.as_ref().map_or((0.0, 0.0), |p| {
                                            let min_latency = p
                                                .conns
                                                .iter()
                                                .filter_map(|c| {
                                                    c.stats.as_ref().map(|s| s.latency_us)
                                                })
                                                .min()
                                                .unwrap_or(0)
                                                as f64
                                                / 1000.0;

                                            let avg_loss = p
                                                .conns
                                                .iter()
                                                .map(|c| c.loss_rate)
                                                .sum::<f32>()
                                                / p.conns.len().max(1) as f32;

                                            (min_latency, avg_loss as f64)
                                        });

                                    hops.push(NodeHopStats {
                                        peer_id: route.peer_id,
                                        target_ip: target_ip.clone(),
                                        latency_ms: latency,
                                        packet_loss: loss as f32,
                                        node_name: route.hostname.clone(),
                                    });
                                }
                            }
                        }

                        // 如果没有收集到任何跳点（可能是直连节点），则直接添加目标节点
                        // 检查 hops 是否只包含本地节点
                        if hops.len() <= 1 {
                            // 确保目标节点不在hops中（避免重复添加）
                            if !hops.iter().any(|h| h.target_ip == target_ip) {
                                let (latency, loss) = pair.peer.as_ref().map_or((0.0, 0.0), |p| {
                                    let min_latency = p
                                        .conns
                                        .iter()
                                        .filter_map(|c| c.stats.as_ref().map(|s| s.latency_us))
                                        .min()
                                        .unwrap_or(0)
                                        as f64
                                        / 1000.0;

                                    let avg_loss = p.conns.iter().map(|c| c.loss_rate).sum::<f32>()
                                        / p.conns.len().max(1) as f32;

                                    (min_latency, avg_loss as f64)
                                });

                                hops.push(NodeHopStats {
                                    peer_id: route.peer_id,
                                    target_ip: target_ip,
                                    latency_ms: latency,
                                    packet_loss: loss as f32,
                                    node_name: route.hostname.clone(),
                                });
                            }
                        }
                    }
                    hops
                },
                latency_ms: if route.cost == 1 {
                    pair.get_latency_ms().unwrap_or(0.0)
                } else {
                    route.path_latency as f64
                },
                ipv4: ipv4,

                loss_rate: if let Some(peer) = &pair.peer {
                    let mut total_loss_rate = 0.0;
                    for conn in &peer.conns {
                        total_loss_rate += conn.loss_rate;
                    }
                    total_loss_rate
                } else {
                    0.0 // 如果没有连接信息，默认为0
                },
                nat: route.stun_info.as_ref().map_or_else(
                    || "Unknown".to_string(),
                    |stun| {
                        // 使用NatType枚举替代直接匹配数字
                        let nat_type =
                            NatType::try_from(stun.udp_nat_type).unwrap_or(NatType::Unknown);
                        format!("{:?}", nat_type)
                    },
                ),
                connections: Vec::new(),
                version: route.version.clone(),
                cost,
                conn_type: pair.get_udp_nat_type(),
                tunnel_proto: pair
                    .get_conn_protos()
                    .unwrap_or_default()
                    .join(",")
                    .to_string(),
                rx_bytes: pair.get_rx_bytes().unwrap_or_default(),
                tx_bytes: pair.get_tx_bytes().unwrap_or_default(),
                proxy_cidrs: route.proxy_cidrs.clone(),
            };

            // 收集连接统计信息
            if let Some(peer) = &pair.peer {
                for conn in &peer.conns {
                    if let Some(stats) = &conn.stats {
                        let conn_type = if let Some(tunnel) = &conn.tunnel {
                            tunnel.tunnel_type.clone()
                        } else {
                            "未知".to_string()
                        };

                        node_info.connections.push(KVNodeConnectionStats {
                            conn_type,
                            rx_bytes: stats.rx_bytes,
                            tx_bytes: stats.tx_bytes,
                            rx_packets: stats.rx_packets,
                            tx_packets: stats.tx_packets,
                        });
                    }
                }
            }

            nodes.push(node_info);
        }
    }

    KVNetworkStatus {
        total_nodes: nodes.len(),
        nodes,
    }
}

pub fn init_app() {
    lazy_static::initialize(&RT);
}
