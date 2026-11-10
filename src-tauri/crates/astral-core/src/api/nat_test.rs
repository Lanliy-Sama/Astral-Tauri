use easytier::proto::common::NatType as EasyNatType;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;

/* NAT 类型检测 —— RFC5389 STUN + RFC3489/5780 行为探测的自实现版本。
 * EasyTier 2.7 把 UdpNatTypeDetector 迁入 core 内部（需要壳私有的
 * native_host_runtime），外部无法再借用；本实现只依赖裸 UDP +
 * 标准属性解析，分类矩阵与经典 STUN（RFC3489 §10.1）一致：
 *   · 主服务器 A 普通探测得映射地址 X1 与 OTHER-ADDRESS（B）；
 *   · 向 A 发 CHANGE-REQUEST(换 IP+端口)：有响应 → Full Cone；
 *   · 向 A 发 CHANGE-REQUEST(仅换端口)：有响应 → Restricted Cone；
 *   · 都无响应：向 B 普通探测得 X2 —— X1==X2 → Port Restricted；
 *     X1!=X2 → Symmetric。
 * IPv6 路径仅做连通性（有 Binding 响应 = OpenInternet）。 */

#[derive(Debug, Clone, PartialEq)]
pub enum NatType {
    OpenInternet,
    FullCone,
    RestrictedCone,
    PortRestrictedCone,
    Symmetric,
    SymmetricUdpFirewall,
    Blocked,
    Unknown,
}

impl NatType {
    pub fn get_description(&self) -> String {
        match self {
            NatType::OpenInternet => "Open Internet".to_string(),
            NatType::FullCone => "Full Cone NAT".to_string(),
            NatType::RestrictedCone => "Restricted Cone NAT".to_string(),
            NatType::PortRestrictedCone => "Port Restricted Cone NAT".to_string(),
            NatType::Symmetric => "Symmetric NAT".to_string(),
            NatType::SymmetricUdpFirewall => "Symmetric UDP Firewall".to_string(),
            NatType::Blocked => "UDP Blocked".to_string(),
            NatType::Unknown => "Unknown".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NetworkTestResult {
    pub nat_type_v4: String,
    pub nat_type_v6: String,
    pub ipv4_latency: i64,
    pub ipv6_latency: i64,
}

/* ---- STUN 协议常量 ---- */
const MAGIC_COOKIE: [u8; 4] = [0x21, 0x12, 0xA4, 0x42];
const ATTR_MAPPED: u16 = 0x0001;
const ATTR_CHANGE_REQUEST: u16 = 0x0003;
const ATTR_XOR_MAPPED: u16 = 0x0020;
const ATTR_OTHER_ADDRESS: u16 = 0x8029;
const ATTR_RESPONSE_ORIGIN: u16 = 0x802b;
const CHANGE_PORT: u32 = 0x0000_0002;
const CHANGE_IP_PORT: u32 = 0x0000_0004;

fn normalize_stun_server(input: &str) -> String {
    let mut server = input.trim().to_string();
    if server.starts_with("stun://") {
        server = server.trim_start_matches("stun://").to_string();
    }

    let has_explicit_port = if server.starts_with('[') {
        server.contains("]:")
    } else {
        server.matches(':').count() == 1
    };

    if has_explicit_port {
        server
    } else {
        format!("{}:3478", server)
    }
}

/// 常用公共 STUN 服务器（旧版取自 EasyTier 内置列表，2.7 不再暴露）
const DEFAULT_STUN_SERVERS: &[&str] = &[
    "stun.miwifi.com:3478",
    "stun.hot-chilli.net:3478",
    "stun.l.google.com:19302",
];

fn build_udp_stun_server_list(preferred_server: &str) -> Vec<String> {
    let mut servers = Vec::<String>::new();

    let preferred = normalize_stun_server(preferred_server);
    if !preferred.is_empty() {
        servers.push(preferred);
    }

    for default_server in DEFAULT_STUN_SERVERS {
        let s = default_server.to_string();
        if !servers.contains(&s) {
            servers.push(s);
        }
    }

    servers
}

fn map_easy_nat_type(nat: EasyNatType) -> NatType {
    match nat {
        EasyNatType::OpenInternet | EasyNatType::NoPat => NatType::OpenInternet,
        EasyNatType::FullCone => NatType::FullCone,
        EasyNatType::Restricted => NatType::RestrictedCone,
        EasyNatType::PortRestricted => NatType::PortRestrictedCone,
        EasyNatType::SymUdpFirewall => NatType::SymmetricUdpFirewall,
        EasyNatType::Symmetric | EasyNatType::SymmetricEasyInc | EasyNatType::SymmetricEasyDec => {
            NatType::Symmetric
        }
        EasyNatType::Unknown => NatType::Unknown,
    }
}

/// 组装 Binding Request（change_flags = Some 时附带 CHANGE-REQUEST 属性）
fn create_binding_request(change_flags: Option<u32>) -> Vec<u8> {
    let mut attrs: Vec<u8> = Vec::new();
    if let Some(flags) = change_flags {
        attrs.extend_from_slice(&ATTR_CHANGE_REQUEST.to_be_bytes());
        attrs.extend_from_slice(&4u16.to_be_bytes());
        attrs.extend_from_slice(&flags.to_be_bytes());
    }
    let mut request = vec![0u8; 20 + attrs.len()];
    request[0] = 0x00;
    request[1] = 0x01; // Binding Request
    let len = attrs.len() as u16;
    request[2..4].copy_from_slice(&len.to_be_bytes());
    request[4..8].copy_from_slice(&MAGIC_COOKIE);
    let now_nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    request[8..20].copy_from_slice(&now_nanos.to_be_bytes()[4..16]);
    request[20..].copy_from_slice(&attrs);
    request
}

fn is_success_stun_response(data: &[u8]) -> bool {
    data.len() >= 20 && data[0] == 0x01 && data[1] == 0x01 && data[4..8] == MAGIC_COOKIE
}

/// 解析 Binding 响应：映射地址（XOR 优先，MAPPED 兜底）+ OTHER/RESPONSE-ORIGIN
fn parse_stun_response(data: &[u8]) -> (Option<([u8; 4], u16)>, Option<(String, u16)>) {
    let mut mapped = None;
    let mut other = None;
    if data.len() < 20 {
        return (mapped, other);
    }
    let len = u16::from_be_bytes([data[2], data[3]]) as usize;
    let end = (20 + len).min(data.len());
    let mut off = 20usize;
    while off + 4 <= end {
        let attr = u16::from_be_bytes([data[off], data[off + 1]]);
        let alen = u16::from_be_bytes([data[off + 2], data[off + 3]]) as usize;
        let body = &data[(off + 4).min(end)..(off + 4 + alen).min(end)];
        match attr {
            ATTR_XOR_MAPPED if alen >= 8 && body[0] == 0x00 && body[1] == 0x01 => {
                let port = u16::from_be_bytes([body[2], body[3]]) ^ 0x2112;
                let mut host = [0u8; 4];
                for i in 0..4 {
                    host[i] = body[4 + i] ^ MAGIC_COOKIE[i];
                }
                mapped = Some((host, port));
            }
            ATTR_MAPPED if alen >= 8 && body[0] == 0x00 && body[1] == 0x01 => {
                if mapped.is_none() {
                    let port = u16::from_be_bytes([body[2], body[3]]);
                    let host = [body[4], body[5], body[6], body[7]];
                    mapped = Some((host, port));
                }
            }
            ATTR_OTHER_ADDRESS if alen >= 8 && body[0] == 0x00 && body[1] == 0x01 => {
                let port = u16::from_be_bytes([body[2], body[3]]);
                let host = format!("{}.{}.{}.{}", body[4], body[5], body[6], body[7]);
                other = Some((host, port));
            }
            ATTR_RESPONSE_ORIGIN if alen >= 8 && body[0] == 0x00 && body[1] == 0x01 => {
                if other.is_none() {
                    let port = u16::from_be_bytes([body[2], body[3]]);
                    let host = format!("{}.{}.{}.{}", body[4], body[5], body[6], body[7]);
                    other = Some((host, port));
                }
            }
            _ => {}
        }
        off += 4 + ((alen + 3) & !3); // TLV 按 4 字节对齐
    }
    (mapped, other)
}

/// 同一 socket 上发送并等待事务 ID 匹配的响应
async fn stun_exchange(
    socket: &UdpSocket,
    target: &str,
    request: &[u8],
    timeout_ms: u64,
) -> Result<Vec<u8>, String> {
    let addr: std::net::SocketAddr = target
        .parse()
        .or_else(|| {
            let mut picked = None;
            if let Ok(addrs) = target.to_socket_addrs() {
                for a in addrs {
                    if a.is_ipv4() {
                        picked = Some(a);
                        break;
                    }
                }
            }
            picked.ok_or_else(|| format!("无法解析 {target}"))
        })
        .map_err(|e| format!("地址解析失败: {e}"))?;

    socket
        .send_to(request, addr)
        .await
        .map_err(|e| format!("发送失败: {e}"))?;

    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let mut buf = vec![0u8; 2048];
    loop {
        let Some(remain) = deadline.checked_duration_since(Instant::now()) else {
            return Err("STUN 响应超时".to_string());
        };
        match tokio::time::timeout(remain, socket.recv_from(&mut buf)).await {
            Err(_) => return Err("STUN 响应超时".to_string()),
            Ok(Err(e)) => return Err(format!("接收失败: {e}")),
            Ok(Ok((len, _))) => {
                // 事务 ID（cookie 之后的 12 字节）匹配才算本事务的响应
                if len >= 20 && buf[8..20] == request[8..20] {
                    return Ok(buf[..len].to_vec());
                }
                // 杂包忽略，继续等
            }
        }
    }
}

/// RFC3489 行为探测 + 分类（IPv4）
async fn detect_nat_type_ipv4(stun_server: &str) -> Result<NatType, String> {
    let stun_servers = build_udp_stun_server_list(stun_server);
    if stun_servers.is_empty() {
        return Err("No STUN servers available".to_string());
    }

    let mut last_err = String::from("未探测");
    for server in &stun_servers {
        let socket = match UdpSocket::bind("0.0.0.0:0").await {
            Ok(s) => s,
            Err(e) => return Err(format!("绑定 UDP 失败: {e}")),
        };
        match detect_with_server(&socket, server).await {
            Ok(nat) => return Ok(nat),
            Err(e) => last_err = format!("{server}: {e}"),
        }
    }
    Err(format!("NAT detection failed: {last_err}"))
}

async fn detect_with_server(socket: &UdpSocket, server: &str) -> Result<NatType, String> {
    // 1. 普通探测：X1 + OTHER-ADDRESS（B 服务器地址）
    let req0 = create_binding_request(None);
    let resp0 = stun_exchange(socket, server, &req0, 3000).await?;
    let (x1, other) = parse_stun_response(&resp0);
    let Some((x1_ip, x1_port)) = x1 else {
        return Err("服务器未返回映射地址".to_string());
    };

    let target_b = other.map(|(h, p)| format!("{h}:{p}"));

    // 2. 换 IP+端口探测：有响应 = NAT 转发任意来源 → Full Cone
    let req_cip = create_binding_request(Some(CHANGE_IP_PORT));
    if stun_exchange(socket, server, &req_cip, 2500).await.is_ok() {
        return Ok(NatType::FullCone);
    }

    // 3. 仅换端口探测：有响应 = 只按 IP 过滤 → Restricted Cone
    let req_cp = create_binding_request(Some(CHANGE_PORT));
    if stun_exchange(socket, server, &req_cp, 2500).await.is_ok() {
        return Ok(NatType::RestrictedCone);
    }

    // 4. 两个探测都失败：用 B（不同 IP）复核映射对称性
    if let Some(b) = &target_b {
        if b != server {
            if let Ok(resp) = stun_exchange(socket, b, &req0, 2500).await {
                let (x2, _) = parse_stun_response(&resp);
                if let Some((x2_ip, x2_port)) = x2 {
                    return Ok(if x2_ip == x1_ip && x2_port == x1_port {
                        NatType::PortRestrictedCone
                    } else {
                        NatType::Symmetric
                    });
                }
            }
        }
    }

    // B 也不可达：无法区分 PortRestricted/Symmetric，按最常见保守值返回
    Ok(NatType::PortRestrictedCone)
}

async fn test_udp_ipv4(stun_server: &str) -> (bool, i64, NatType) {
    let start = Instant::now();

    match detect_nat_type_ipv4(stun_server).await {
        Ok(nat_type) => (true, start.elapsed().as_millis() as i64, nat_type),
        Err(_) => (false, -1, NatType::Blocked),
    }
}

async fn test_udp_ipv6(stun_server: &str) -> (bool, i64, NatType) {
    let start = Instant::now();
    let server = normalize_stun_server(stun_server);

    let mut addrs = match tokio::net::lookup_host(&server).await {
        Ok(addrs) => addrs,
        Err(_) => return (false, -1, NatType::Unknown),
    };

    let Some(server_v6) = addrs.find(|addr| addr.is_ipv6()) else {
        return (false, -1, NatType::Unknown);
    };

    let socket = match UdpSocket::bind("[::]:0").await {
        Ok(s) => s,
        Err(_) => return (false, -1, NatType::Unknown),
    };

    let request = create_binding_request(None);
    if socket.send_to(&request, server_v6).await.is_err() {
        return (false, -1, NatType::Unknown);
    }

    let mut buf = [0u8; 1500];
    let recv = tokio::time::timeout(Duration::from_secs(3), socket.recv_from(&mut buf)).await;

    match recv {
        Ok(Ok((len, _))) if is_success_stun_response(&buf[..len]) => (
            true,
            start.elapsed().as_millis() as i64,
            NatType::OpenInternet,
        ),
        _ => (false, -1, NatType::Unknown),
    }
}

/// NAT test entry for Flutter.
pub fn test_network_connectivity(stun_server: String) -> Result<NetworkTestResult, String> {
    let rt =
        tokio::runtime::Runtime::new().map_err(|e| format!("Failed to create runtime: {e}"))?;

    let result = rt.block_on(async {
        let (v4, v6) = tokio::join!(test_udp_ipv4(&stun_server), test_udp_ipv6(&stun_server));

        NetworkTestResult {
            nat_type_v4: v4.2.get_description(),
            nat_type_v6: v6.2.get_description(),
            ipv4_latency: v4.1,
            ipv6_latency: v6.1,
        }
    });

    Ok(result)
}

/// Backward-compatible API: detect IPv4 NAT type.
pub fn detect_nat_type(stun_server: String) -> Result<String, String> {
    let rt =
        tokio::runtime::Runtime::new().map_err(|e| format!("Failed to create runtime: {e}"))?;

    let nat_type = rt.block_on(async { detect_nat_type_ipv4(&stun_server).await })?;
    Ok(nat_type.get_description())
}
