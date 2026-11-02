//! serde 序列化镜像类型 —— 供 Tauri 命令层使用。
//! 字段命名与前端 `src/lib/rust/api.js` 期望的 JSON 形状完全一致（camelCase）。

use serde::{Deserialize, Serialize};

/// KVNetworkStatus 的可序列化镜像（前端直接消费）
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStatusDto {
    pub total_nodes: u64,
    pub nodes: Vec<NodeInfoDto>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfoDto {
    pub peer_id: u32,
    pub hostname: String,
    pub ipv4: String,
    pub latency_ms: f64,
    pub nat: String,
    pub hops: Vec<NodeHopStatsDto>,
    pub loss_rate: f32,
    pub connections: Vec<NodeConnectionStatsDto>,
    pub tunnel_proto: String,
    pub conn_type: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub version: String,
    pub cost: i32,
    pub proxy_cidrs: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeHopStatsDto {
    pub peer_id: u32,
    pub target_ip: String,
    pub latency_ms: f64,
    pub packet_loss: f32,
    pub node_name: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeConnectionStatsDto {
    pub conn_type: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
}

/// NAT 测试结果镜像
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkTestResultDto {
    pub nat_type_v4: String,
    pub nat_type_v6: String,
    pub ipv4_latency: i64,
    pub ipv6_latency: i64,
}

/// create_server 的配置参数（前端 ServerConfigBuilder 输出的镜像）。
/// 注意 `severurl` 为原版拼写，保持一致。
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateServerConfig {
    pub username: String,
    pub enable_dhcp: bool,
    pub specified_ip: String,
    pub room_name: String,
    pub room_password: String,
    pub severurl: Vec<String>,
    pub onurl: Vec<String>,
    pub cidrs: Vec<String>,
    #[allow(dead_code)]
    pub forwards: Vec<ForwardDto>,
    pub flag: FlagsDto,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardDto {
    #[allow(dead_code)]
    pub bind_addr: String,
    #[allow(dead_code)]
    pub dst_addr: String,
    #[allow(dead_code)]
    pub proto: String,
}

/// 运行标志（FlagsC 镜像）
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlagsDto {
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
    pub socks5_port: u16,
}
