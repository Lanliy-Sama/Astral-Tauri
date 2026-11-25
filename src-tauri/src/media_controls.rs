//! 系统媒体控制（Windows SMTC / Linux MPRIS / macOS Now Playing）。
//! 控件在专属线程创建和销毁，前端只提交可序列化播放状态。
use serde_json::{json, Value};
use tauri::{Emitter, Manager};

#[derive(Clone, Debug)]
pub struct MediaUpdate {
    enabled: bool,
    title: String,
    artist: String,
    album: String,
    cover: String,
    duration_ms: u64,
    position_ms: u64,
    playing: bool,
    volume: f64,
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn smtc_update(
    enabled: bool,
    title: String,
    artist: String,
    album: String,
    cover: String,
    duration_ms: u64,
    position_ms: u64,
    playing: bool,
    volume: f64,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let update = MediaUpdate {
        enabled, title, artist, album, cover, duration_ms, position_ms, playing,
        volume: volume.clamp(0.0, 1.0),
    };
    #[cfg(not(target_os = "android"))]
    {
        #[cfg(target_os = "windows")]
        let hwnd = app.get_webview_window("main")
            .and_then(|window| window.hwnd().ok())
            .map(|handle| handle.0 as usize);
        #[cfg(not(target_os = "windows"))]
        let hwnd = None;
        system::submit(update, hwnd, app)
    }
    #[cfg(target_os = "android")]
    {
        let _ = (update, app);
        Err("Android 请使用系统媒体通知，当前版本尚未启用".into())
    }
}

#[cfg(not(target_os = "android"))]
mod system {
    use super::*;
    use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig};
    use std::sync::mpsc::{channel, Sender};
    use std::sync::{Mutex, OnceLock};
    use std::time::Duration;

    static SENDER: OnceLock<Mutex<Option<Sender<MediaUpdate>>>> = OnceLock::new();

    fn event_json(event: MediaControlEvent) -> Option<Value> {
        match event {
            MediaControlEvent::Play => Some(json!({ "kind": "play" })),
            MediaControlEvent::Pause => Some(json!({ "kind": "pause" })),
            MediaControlEvent::Toggle => Some(json!({ "kind": "toggle" })),
            MediaControlEvent::Next => Some(json!({ "kind": "next" })),
            MediaControlEvent::Previous => Some(json!({ "kind": "previous" })),
            MediaControlEvent::Stop => Some(json!({ "kind": "stop" })),
            MediaControlEvent::SetPosition(position) => Some(json!({
                "kind": "position", "positionMs": position.0.as_millis() as u64,
            })),
            MediaControlEvent::SetVolume(volume) => Some(json!({ "kind": "volume", "volume": volume })),
            _ => None,
        }
    }

    pub fn submit(update: MediaUpdate, hwnd: Option<usize>, app: tauri::AppHandle) -> Result<(), String> {
        let sender = SENDER.get_or_init(|| Mutex::new(None));
        let mut guard = sender.lock().unwrap_or_else(|e| e.into_inner());
        if guard.is_none() {
            let (tx, rx) = channel::<MediaUpdate>();
            std::thread::spawn(move || {
                let mut controls: Option<MediaControls> = None;
                while let Ok(update) = rx.recv() {
                    if !update.enabled {
                        controls = None;
                        continue;
                    }
                    if controls.is_none() {
                        let config = PlatformConfig {
                            display_name: "Astral",
                            dbus_name: "astral",
                            hwnd: hwnd.map(|value| value as *mut std::ffi::c_void),
                        };
                        match MediaControls::new(config) {
                            Ok(mut created) => {
                                let event_app = app.clone();
                                let attached = created.attach(move |event| {
                                    if let Some(payload) = event_json(event) {
                                        let _ = event_app.emit("smtc-event", payload);
                                    }
                                });
                                if let Err(error) = attached {
                                    let _ = app.emit("smtc-error", error.to_string());
                                    continue;
                                }
                                controls = Some(created);
                            }
                            Err(error) => {
                                let _ = app.emit("smtc-error", error.to_string());
                                continue;
                            }
                        }
                    }
                    if let Some(control) = controls.as_mut() {
                        let duration = (update.duration_ms > 0).then(|| Duration::from_millis(update.duration_ms));
                        let position = Some(MediaPosition(Duration::from_millis(update.position_ms)));
                        let playback = if update.playing {
                            MediaPlayback::Playing { progress: position }
                        } else {
                            MediaPlayback::Paused { progress: position }
                        };
                        if let Err(error) = control.set_metadata(MediaMetadata {
                            title: (!update.title.is_empty()).then_some(update.title.as_str()),
                            artist: (!update.artist.is_empty()).then_some(update.artist.as_str()),
                            album: (!update.album.is_empty()).then_some(update.album.as_str()),
                            cover_url: (!update.cover.is_empty()).then_some(update.cover.as_str()),
                            duration,
                        }) {
                            let _ = app.emit("smtc-error", error.to_string());
                        }
                        let _ = control.set_playback(playback);
                        #[cfg(not(target_os = "windows"))]
                        let _ = control.set_volume(update.volume);
                    }
                }
            });
            *guard = Some(tx);
        }
        guard.as_ref().unwrap().send(update).map_err(|e| e.to_string())
    }
}
