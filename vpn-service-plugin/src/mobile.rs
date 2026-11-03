use serde::de::DeserializeOwned;
use tauri::{
    plugin::{PluginApi, PluginHandle},
    AppHandle, Runtime,
};

use crate::models::*;

const PLUGIN_IDENTIFIER: &str = "com.plugin.vpnservice";

pub struct Vpnservice<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> Vpnservice<R> {
    pub fn prepare_vpn(&self) -> Result<Status, String> {
        self.0
            .run_mobile_plugin("prepare_vpn", VoidRequest {})
            .map_err(|e| e.to_string())
    }

    pub fn start_vpn(&self, req: StartVpnRequest) -> Result<Status, String> {
        self.0
            .run_mobile_plugin("start_vpn", req)
            .map_err(|e| e.to_string())
    }

    pub fn stop_vpn(&self) -> Result<Status, String> {
        self.0
            .run_mobile_plugin("stop_vpn", VoidRequest {})
            .map_err(|e| e.to_string())
    }
}

pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> Result<Vpnservice<R>, String> {
    let handle = api
        .register_android_plugin(PLUGIN_IDENTIFIER, "VpnServicePlugin")
        .map_err(|e| e.to_string())?;
    Ok(Vpnservice(handle))
}
