use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VoidRequest {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartVpnRequest {
    pub ipv4_addr: Option<String>,
    pub routes: Option<Vec<String>>,
    pub dns: Option<String>,
    pub disallowed_applications: Option<Vec<String>>,
    pub mtu: Option<u32>,
}

/// 统一返回：`error_msg` 表示失败原因，`granted` 仅授权接口使用，
/// `fd` 仅 `start_vpn` 使用。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_msg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub granted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fd: Option<i32>,
}
