//! Android VpnService 桥接插件（基于 EasyTier 官方 tauri-plugin-vpnservice 适配）。
//!
//! 仅 Android 平台注册状态；其他平台 `init()` 为空操作，由应用层命令返回「不支持」。

use tauri::{
    plugin::{Builder, TauriPlugin},
    Runtime,
};

#[cfg(target_os = "android")]
use tauri::Manager;

pub mod models;

#[cfg(target_os = "android")]
mod mobile;

#[cfg(target_os = "android")]
pub use mobile::Vpnservice;

#[cfg(target_os = "android")]
pub trait VpnserviceExt<R: Runtime> {
    fn vpnservice(&self) -> &Vpnservice<R>;
}

#[cfg(target_os = "android")]
impl<R: Runtime, T: Manager<R>> VpnserviceExt<R> for T {
    fn vpnservice(&self) -> &Vpnservice<R> {
        self.state::<Vpnservice<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("vpnservice")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                let vpnservice = mobile::init(app, api)?;
                app.manage(vpnservice);
            }
            #[cfg(not(target_os = "android"))]
            let _ = (app, api);
            Ok(())
        })
        .build()
}
