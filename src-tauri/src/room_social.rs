//! 房间应用层消息与账号端音频代理。
//! 控制消息仅走 EasyTier 虚拟 IP；登录 Cookie/token 永不进入消息或流协议。
use serde_json::{json, Value};
use base64::Engine as _;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::Emitter;

const SOCIAL_PORT: u16 = 43879;
const MAX_MESSAGE: usize = 1024 * 1024;
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/124.0 Safari/537.36";

static ROOM: OnceLock<Mutex<String>> = OnceLock::new();
static SOCIAL_RUNNING: AtomicBool = AtomicBool::new(false);
static STREAM_RUNNING: AtomicBool = AtomicBool::new(false);
static PROXY_RUNNING: AtomicBool = AtomicBool::new(false);
static STREAM_PORT: OnceLock<Mutex<u16>> = OnceLock::new();
static PROXY_PORT: OnceLock<Mutex<u16>> = OnceLock::new();
static OFFERS: OnceLock<Mutex<HashMap<String, StreamOffer>>> = OnceLock::new();
static SHARED_FILES: OnceLock<Mutex<HashMap<String, SharedFile>>> = OnceLock::new();
static FILE_WRITES: OnceLock<Mutex<HashMap<String, FileWrite>>> = OnceLock::new();
static MEDIA_RUNNING: AtomicBool = AtomicBool::new(false);
static MEDIA_PORT: OnceLock<Mutex<u16>> = OnceLock::new();
static MEDIA_SINKS: OnceLock<Mutex<HashMap<String, TcpStream>>> = OnceLock::new();

#[derive(Clone)]
struct StreamOffer {
    url: String,
    platform: String,
    format: String,
    content_type: String,
    file_path: Option<std::path::PathBuf>,
    created: Instant,
}

#[derive(Clone)]
struct SharedFile {
    path: std::path::PathBuf,
    name: String,
    size: u64,
    mime: String,
}

struct FileWrite {
    file: std::fs::File,
    path: std::path::PathBuf,
    name: String,
    size: u64,
    mime: String,
    written: u64,
}

fn room() -> &'static Mutex<String> {
    ROOM.get_or_init(|| Mutex::new(String::new()))
}
fn offers() -> &'static Mutex<HashMap<String, StreamOffer>> {
    OFFERS.get_or_init(|| Mutex::new(HashMap::new()))
}
fn shared_files() -> &'static Mutex<HashMap<String, SharedFile>> {
    SHARED_FILES.get_or_init(|| Mutex::new(HashMap::new()))
}
fn file_writes() -> &'static Mutex<HashMap<String, FileWrite>> {
    FILE_WRITES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn read_line(stream: &mut TcpStream, limit: usize) -> Result<String, String> {
    let mut data = Vec::new();
    let mut byte = [0u8; 1];
    while data.len() < limit {
        let count = stream.read(&mut byte).map_err(|e| e.to_string())?;
        if count == 0 { return Err("连接已关闭".into()); }
        if byte[0] == b'\n' {
            return String::from_utf8(data).map_err(|e| e.to_string());
        }
        data.push(byte[0]);
    }
    Err("消息头过长".into())
}

fn handle_social(app: tauri::AppHandle, mut stream: TcpStream, peer: SocketAddr) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let Ok(line) = read_line(&mut stream, MAX_MESSAGE) else { return };
    let Ok(message) = serde_json::from_str::<Value>(line.trim_end_matches('\r')) else { return };
    let Some(kind) = message["kind"].as_str() else { return };
    if kind.len() > 48 || message["room"].as_str() != Some(room().lock().unwrap_or_else(|e| e.into_inner()).as_str()) {
        return;
    }
    let _ = app.emit("room-social", json!({ "message": message, "peer": peer.ip().to_string() }));
}

#[tauri::command]
pub fn room_social_start(room_id: String, app: tauri::AppHandle) -> Result<Value, String> {
    if room_id.len() > 256 || room_id.trim().is_empty() {
        return Err("房间标识无效".into());
    }
    *room().lock().unwrap_or_else(|e| e.into_inner()) = room_id;
    if SOCIAL_RUNNING.swap(true, Ordering::SeqCst) {
        return Ok(json!({ "port": SOCIAL_PORT }));
    }
    let listener = TcpListener::bind(("0.0.0.0", SOCIAL_PORT)).map_err(|e| {
        SOCIAL_RUNNING.store(false, Ordering::SeqCst);
        format!("房间消息端口监听失败: {e}")
    })?;
    std::thread::spawn(move || {
        for incoming in listener.incoming() {
            let Ok(stream) = incoming else { continue };
            let Ok(peer) = stream.peer_addr() else { continue };
            let handle = app.clone();
            std::thread::spawn(move || handle_social(handle, stream, peer));
        }
        SOCIAL_RUNNING.store(false, Ordering::SeqCst);
    });
    Ok(json!({ "port": SOCIAL_PORT }))
}

#[tauri::command]
pub fn room_social_stop() {
    *room().lock().unwrap_or_else(|e| e.into_inner()) = String::new();
}

#[tauri::command]
pub fn room_social_send(peer: String, message: Value) -> Result<(), String> {
    let current = room().lock().unwrap_or_else(|e| e.into_inner()).clone();
    if current.is_empty() || message["room"].as_str() != Some(current.as_str()) {
        return Err("房间消息会话已结束".into());
    }
    if message["kind"].as_str().map(|s| s.len() > 48).unwrap_or(true) {
        return Err("消息类型无效".into());
    }
    let ip: std::net::IpAddr = peer.parse().map_err(|_| "房员 IP 无效".to_string())?;
    let address = SocketAddr::new(ip, SOCIAL_PORT);
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_millis(1800))
        .map_err(|e| format!("房员连接失败: {e}"))?;
    stream.set_write_timeout(Some(Duration::from_secs(3))).map_err(|e| e.to_string())?;
    let mut data = serde_json::to_vec(&message).map_err(|e| e.to_string())?;
    if data.len() > MAX_MESSAGE { return Err("消息过大".into()); }
    data.push(b'\n');
    stream.write_all(&data).map_err(|e| e.to_string())
}

fn ensure_stream_listener() -> Result<u16, String> {
    if let Some(port) = STREAM_PORT.get().and_then(|p| p.lock().ok().map(|g| *g)) {
        return Ok(port);
    }
    let listener = TcpListener::bind(("0.0.0.0", 0)).map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    if STREAM_RUNNING.swap(true, Ordering::SeqCst) { return Ok(port); }
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            std::thread::spawn(move || serve_stream(stream));
        }
        STREAM_RUNNING.store(false, Ordering::SeqCst);
    });
    let mut slot = STREAM_PORT.get_or_init(|| Mutex::new(0)).lock().unwrap_or_else(|e| e.into_inner());
    *slot = port;
    Ok(port)
}

fn upstream_headers(platform: &str) -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(reqwest::header::USER_AGENT, reqwest::header::HeaderValue::from_static(UA));
    let referer = match platform {
        "netease" => "https://music.163.com/",
        "qq" => "https://y.qq.com/",
        _ => "https://www.kugou.com/",
    };
    headers.insert(reqwest::header::REFERER, reqwest::header::HeaderValue::from_static(referer));
    headers
}

fn serve_file_stream(stream: &mut TcpStream, path: &std::path::Path, range: &str, content_type: &str) -> Result<(), String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let total = file.metadata().map_err(|e| e.to_string())?.len();
    let mut start = 0u64;
    let mut end = total.saturating_sub(1);
    if let Some(spec) = range.strip_prefix("bytes=") {
        let mut parts = spec.splitn(2, '-');
        start = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        if let Some(value) = parts.next().and_then(|v| v.parse().ok()) {
            end = value.min(total.saturating_sub(1));
        }
    }
    if start > end || start >= total {
        stream.write_all(b"{\"status\":416}\n").map_err(|e| e.to_string())?;
        return Ok(());
    }
    let length = end - start + 1;
    let status = if start > 0 || end + 1 < total { 206 } else { 200 };
    let content_range = format!("bytes {start}-{end}/{total}");
    let metadata = json!({
        "status": status,
        "contentType": content_type,
        "contentLength": length,
        "contentRange": content_range,
        "acceptRanges": "bytes",
    });
    stream.write_all(metadata.to_string().as_bytes()).map_err(|e| e.to_string())?;
    stream.write_all(b"\n").map_err(|e| e.to_string())?;
    file.seek(SeekFrom::Start(start)).map_err(|e| e.to_string())?;
    std::io::copy(&mut file.take(length), stream).map_err(|e| e.to_string())?;
    Ok(())
}

fn serve_stream(mut stream: TcpStream) {
    let result = (|| -> Result<(), String> {
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let line = read_line(&mut stream, 4096)?;
        let request: Value = serde_json::from_str(line.trim_end_matches('\r')).map_err(|e| e.to_string())?;
        let id = request["id"].as_str().unwrap_or_default();
        if id.len() != 36 { return Err("流标识无效".into()); }
        let offer = offers().lock().unwrap_or_else(|e| e.into_inner()).get(id).cloned()
            .ok_or("音频流已过期，请重新播放")?;
        let range = request["range"].as_str().unwrap_or_default();
        if let Some(path) = offer.file_path.as_deref() {
            return serve_file_stream(&mut stream, path, range, &offer.content_type);
        }
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(8))
            // 不设置整个响应的总超时：歌曲可能持续数分钟，流式拷贝期间保持连接。
            .build().map_err(|e| e.to_string())?;
        let mut call = client.get(&offer.url).headers(upstream_headers(&offer.platform));
        if range.starts_with("bytes=") && range.len() < 64 && range.chars().all(|c| c.is_ascii_digit() || "bytes=,-".contains(c)) {
            call = call.header(reqwest::header::RANGE, range);
        }
        let mut response = call.send().map_err(|e| format!("账号端音频请求失败: {e}"))?;
        let status = response.status();
        let content_type = response.headers().get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()).unwrap_or(if offer.format == "flac" { "audio/flac" } else { "audio/mpeg" }).to_string();
        let metadata = json!({
            "status": status.as_u16(),
            "contentType": content_type,
            "contentLength": response.content_length(),
            "contentRange": response.headers().get(reqwest::header::CONTENT_RANGE).and_then(|v| v.to_str().ok()),
            "acceptRanges": response.headers().get(reqwest::header::ACCEPT_RANGES).and_then(|v| v.to_str().ok()).unwrap_or("bytes"),
        });
        let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));
        stream.write_all(metadata.to_string().as_bytes()).map_err(|e| e.to_string())?;
        stream.write_all(b"\n").map_err(|e| e.to_string())?;
        if status.is_success() {
            std::io::copy(&mut response, &mut stream).map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = stream.write_all(b"{\"status\":502}\n");
    }
}

#[tauri::command]
pub fn music_stream_offer(track: Value, app: tauri::AppHandle) -> Result<Value, String> {
    let platform = track["platform"].as_str().unwrap_or_default().to_string();
    let key = track["key"].as_str().unwrap_or_default().to_string();
    if platform.is_empty() || key.is_empty() { return Err("共享曲目信息无效".into()); }
    // 只在账号端解析，不调用前端 Audio，账号端不会出声。
    let source = crate::music_platform::music_play_url(platform.clone(), key.clone(), Some(track.clone()), app)?;
    let url = source["url"].as_str().unwrap_or_default().to_string();
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err("账号端没有取得有效音频".into());
    }
    let port = ensure_stream_listener()?;
    let id = uuid::Uuid::new_v4().to_string();
    let format = source["format"].as_str().unwrap_or("mp3").to_string();
    let mut map = offers().lock().unwrap_or_else(|e| e.into_inner());
    map.retain(|_, value| value.created.elapsed() < Duration::from_secs(600));
    map.insert(id.clone(), StreamOffer { url, platform: platform.clone(), format: format.clone(), content_type: "audio/mpeg".into(), file_path: None, created: Instant::now() });
    drop(map);
    let lyrics = crate::music_platform::music_lyrics(platform, key, Some(track)).ok();
    Ok(json!({ "streamId": id, "streamPort": port, "format": format, "lyrics": lyrics }))
}

#[tauri::command]
pub fn local_music_stream_offer(track_id: String, app: tauri::AppHandle) -> Result<Value, String> {
    use tauri::Manager;
    if track_id.len() != 36 || track_id.contains('/') || track_id.contains('\\') || track_id.contains("..") {
        return Err("本地曲目标识无效".into());
    }
    let directory = app.path().app_data_dir().map_err(|e| e.to_string())?.join("bgm");
    let path = std::fs::read_dir(&directory)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.file_name().map(|name| name.to_string_lossy().starts_with(&format!("{track_id}."))).unwrap_or(false))
        .ok_or("本地音频文件不存在")?;
    let format = path.extension().map(|ext| ext.to_string_lossy().to_string()).unwrap_or_else(|| "mp3".into());
    let port = ensure_stream_listener()?;
    let id = uuid::Uuid::new_v4().to_string();
    offers().lock().unwrap_or_else(|e| e.into_inner()).insert(id.clone(), StreamOffer {
        url: String::new(), platform: "local".into(), format: format.clone(),
        content_type: if format == "flac" { "audio/flac".into() } else { "audio/mpeg".into() },
        file_path: Some(path), created: Instant::now(),
    });
    Ok(json!({ "streamId": id, "streamPort": port, "format": format, "lyrics": { "found": false, "lines": [] } }))
}

fn shared_file_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    use tauri::Manager;
    let directory = app.path().app_data_dir().map_err(|e| e.to_string())?.join("shared_files");
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    Ok(directory)
}

#[tauri::command]
pub fn room_file_write_begin(name: String, size: u64, mime: String, app: tauri::AppHandle) -> Result<String, String> {
    if name.trim().is_empty() || name.len() > 255 || size > 2 * 1024 * 1024 * 1024 || mime.len() > 128 {
        return Err("文件元数据无效或文件超过 2GB".into());
    }
    let id = uuid::Uuid::new_v4().to_string();
    let extension = std::path::Path::new(&name).extension()
        .map(|ext| ext.to_string_lossy().to_string())
        .filter(|ext| !ext.is_empty() && ext.len() <= 10 && ext.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or_else(|| "bin".into());
    let path = shared_file_dir(&app)?.join(format!("{id}.{extension}"));
    let file = std::fs::File::create_new(&path).map_err(|e| e.to_string())?;
    file_writes().lock().unwrap_or_else(|e| e.into_inner()).insert(id.clone(), FileWrite {
        file, path, name, size, mime, written: 0,
    });
    Ok(id)
}

#[tauri::command]
pub fn room_file_write_append(id: String, b64: String) -> Result<(), String> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64).map_err(|e| e.to_string())?;
    let mut writes = file_writes().lock().unwrap_or_else(|e| e.into_inner());
    let session = writes.get_mut(&id).ok_or("文件写入会话不存在")?;
    if session.written + bytes.len() as u64 > session.size {
        return Err("文件内容超过声明大小".into());
    }
    session.file.write_all(&bytes).map_err(|e| e.to_string())?;
    session.written += bytes.len() as u64;
    Ok(())
}

#[tauri::command]
pub fn room_file_write_end(id: String) -> Result<Value, String> {
    let session = file_writes().lock().unwrap_or_else(|e| e.into_inner()).remove(&id).ok_or("文件写入会话不存在")?;
    if session.written != session.size {
        return Err("文件尚未写入完成".into());
    }
    session.file.sync_all().map_err(|e| e.to_string())?;
    let metadata = json!({
        "id": id, "name": session.name, "size": session.size, "mime": session.mime,
    });
    shared_files().lock().unwrap_or_else(|e| e.into_inner()).insert(id, SharedFile {
        path: session.path,
        name: metadata["name"].as_str().unwrap_or_default().to_string(),
        size: session.size,
        mime: session.mime,
    });
    Ok(metadata)
}

#[tauri::command]
pub fn room_file_offer(file_id: String) -> Result<Value, String> {
    let (path, mime) = {
        let files = shared_files().lock().unwrap_or_else(|e| e.into_inner());
        let file = files.get(&file_id).ok_or("共享文件不存在")?;
        (file.path.clone(), file.mime.clone())
    };
    let port = ensure_stream_listener()?;
    let id = uuid::Uuid::new_v4().to_string();
    offers().lock().unwrap_or_else(|e| e.into_inner()).insert(id.clone(), StreamOffer {
        url: String::new(), platform: "file".into(), format: "file".into(),
        content_type: if mime.is_empty() { "application/octet-stream".into() } else { mime },
        file_path: Some(path), created: Instant::now(),
    });
    Ok(json!({ "streamId": id, "streamPort": port }))
}

#[tauri::command]
pub fn room_file_download(peer: String, stream_port: u16, stream_id: String, file_name: String, app: tauri::AppHandle) -> Result<Value, String> {
    use tauri::Manager;
    let ip: std::net::IpAddr = peer.parse().map_err(|_| "分享端地址无效".to_string())?;
    if stream_id.len() != 36 { return Err("文件流标识无效".into()); }
    let mut remote = TcpStream::connect_timeout(&SocketAddr::new(ip, stream_port), Duration::from_secs(6))
        .map_err(|e| format!("分享端连接失败: {e}"))?;
    remote.write_all(json!({ "id": stream_id, "range": Value::Null }).to_string().as_bytes()).map_err(|e| e.to_string())?;
    remote.write_all(b"\n").map_err(|e| e.to_string())?;
    let metadata_line = read_line(&mut remote, 8192)?;
    let metadata: Value = serde_json::from_str(metadata_line.trim_end_matches('\r')).map_err(|e| e.to_string())?;
    let status = metadata["status"].as_u64().unwrap_or(502);
    if status != 200 && status != 206 {
        return Err(format!("分享端返回错误 {status}"));
    }
    let safe_name: String = file_name.chars()
        .map(|c| if c.is_alphanumeric() || matches!(c, '.' | '_' | '-' | ' ') { c } else { '_' })
        .take(128).collect();
    let directory = app.path().app_data_dir().map_err(|e| e.to_string())?.join("downloads");
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let mut path = directory.join(if safe_name.trim().is_empty() { "download.bin".into() } else { safe_name });
    if path.exists() {
        let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "download".into());
        let ext = path.extension().map(|s| s.to_string_lossy().to_string());
        path = directory.join(format!("{stem}-{}{}", uuid::Uuid::new_v4().simple().to_string().chars().take(8).collect::<String>(), ext.map(|e| format!(".{e}")).unwrap_or_default()));
    }
    let mut file = std::fs::File::create_new(&path).map_err(|e| e.to_string())?;
    let written = std::io::copy(&mut remote, &mut file).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    Ok(json!({ "path": path.to_string_lossy(), "size": written }))
}

fn ensure_proxy_listener() -> Result<u16, String> {
    if let Some(port) = PROXY_PORT.get().and_then(|p| p.lock().ok().map(|g| *g)) {
        return Ok(port);
    }
    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    if PROXY_RUNNING.swap(true, Ordering::SeqCst) { return Ok(port); }
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            std::thread::spawn(move || proxy_stream(stream));
        }
        PROXY_RUNNING.store(false, Ordering::SeqCst);
    });
    let mut slot = PROXY_PORT.get_or_init(|| Mutex::new(0)).lock().unwrap_or_else(|e| e.into_inner());
    *slot = port;
    Ok(port)
}

fn query_value(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|part| {
        let (name, value) = part.split_once('=')?;
        (name == key).then(|| value.to_string())
    })
}

fn proxy_stream(mut local: TcpStream) {
    let result = (|| -> Result<(), String> {
        let _ = local.set_read_timeout(Some(Duration::from_secs(5)));
        let mut head = Vec::new();
        let mut byte = [0u8; 1];
        while head.len() < 16384 {
            let count = local.read(&mut byte).map_err(|e| e.to_string())?;
            if count == 0 { return Err("本地媒体连接关闭".into()); }
            head.push(byte[0]);
            if head.ends_with(b"\r\n\r\n") { break; }
        }
        let text = String::from_utf8_lossy(&head);
        let request_line = text.lines().next().ok_or("媒体请求无效")?;
        let target = request_line.split_whitespace().nth(1).unwrap_or_default();
        let query = target.split_once('?').map(|(_, q)| q).unwrap_or_default();
        let peer = query_value(query, "peer").ok_or("缺少账号端地址")?;
        let port: u16 = query_value(query, "port").and_then(|v| v.parse().ok()).ok_or("缺少流端口")?;
        let id = query_value(query, "id").ok_or("缺少流标识")?;
        let ip: std::net::IpAddr = peer.parse().map_err(|_| "账号端地址无效".to_string())?;
        let range = text.lines().find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("range").then(|| value.trim().to_string())
        });
        let mut remote = TcpStream::connect_timeout(&SocketAddr::new(ip, port), Duration::from_secs(5))
            .map_err(|e| format!("账号端音频流连接失败: {e}"))?;
        remote.write_all(json!({ "id": id, "range": range }).to_string().as_bytes()).map_err(|e| e.to_string())?;
        remote.write_all(b"\n").map_err(|e| e.to_string())?;
        let metadata_line = read_line(&mut remote, 8192)?;
        let metadata: Value = serde_json::from_str(metadata_line.trim_end_matches('\r')).map_err(|e| e.to_string())?;
        let status = metadata["status"].as_u64().unwrap_or(502) as u16;
        let reason = if status == 206 { "Partial Content" } else if status == 200 { "OK" } else { "Bad Gateway" };
        let content_type = metadata["contentType"].as_str().unwrap_or("application/octet-stream");
        let mut response = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nAccess-Control-Allow-Origin: *\r\nAccept-Ranges: bytes\r\nConnection: close\r\n"
        );
        if let Some(length) = metadata["contentLength"].as_u64() {
            response.push_str(&format!("Content-Length: {length}\r\n"));
        }
        if let Some(range) = metadata["contentRange"].as_str() {
            if range.len() < 256 && !range.contains('\r') && !range.contains('\n') {
                response.push_str(&format!("Content-Range: {range}\r\n"));
            }
        }
        response.push_str("\r\n");
        let _ = local.set_write_timeout(Some(Duration::from_secs(30)));
        local.write_all(response.as_bytes()).map_err(|e| e.to_string())?;
        if status == 200 || status == 206 {
            std::io::copy(&mut remote, &mut local).map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = local.write_all(b"HTTP/1.1 502 Bad Gateway\r\nConnection: close\r\nContent-Length: 0\r\n\r\n");
    }
}

#[tauri::command]
pub fn room_music_stream_url(peer: String, stream_port: u16, stream_id: String) -> Result<String, String> {
    let _: std::net::IpAddr = peer.parse().map_err(|_| "账号端地址无效".to_string())?;
    if stream_id.len() != 36 { return Err("流标识无效".into()); }
    let port = ensure_proxy_listener()?;
    Ok(format!("http://127.0.0.1:{port}/?peer={peer}&port={stream_port}&id={stream_id}"))
}

/* ================= 编码媒体 TCP 流（语音 / 一起看 / 远程桌面） ================= */

fn media_sinks() -> &'static Mutex<HashMap<String, TcpStream>> {
    MEDIA_SINKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn handle_media(app: tauri::AppHandle, mut stream: TcpStream, peer: SocketAddr) {
    let result = (|| -> Result<(), String> {
        let header = read_line(&mut stream, 4096)?;
        let metadata: Value = serde_json::from_str(header.trim_end_matches('\r')).map_err(|e| e.to_string())?;
        let session = metadata["session"].as_str().unwrap_or_default();
        let kind = metadata["kind"].as_str().unwrap_or_default();
        let codec = metadata["codec"].as_str().unwrap_or_default();
        if session.is_empty() || session.len() > 160 || !matches!(kind, "voice" | "watch" | "desktop") || codec.len() > 128 {
            return Err("媒体流元数据无效".into());
        }
        loop {
            let mut header = [0u8; 8];
            stream.read_exact(&mut header).map_err(|e| e.to_string())?;
            let length = u32::from_be_bytes(header[..4].try_into().unwrap()) as usize;
            let sequence = u32::from_be_bytes(header[4..].try_into().unwrap());
            if length > 64 * 1024 * 1024 { return Err("媒体分片过大".into()); }
            let mut data = vec![0u8; length];
            stream.read_exact(&mut data).map_err(|e| e.to_string())?;
            let _ = app.emit("room-media", json!({
                "peer": peer.ip().to_string(), "session": session, "kind": kind,
                "codec": codec, "sequence": sequence, "end": length == 0,
                "data": base64::engine::general_purpose::STANDARD.encode(data),
            }));
            if length == 0 { break; }
        }
        Ok(())
    })();
    let _ = result;
}

#[tauri::command]
pub fn room_media_listener_start(app: tauri::AppHandle) -> Result<Value, String> {
    if let Some(port) = MEDIA_PORT.get().and_then(|p| p.lock().ok().map(|g| *g)) {
        return Ok(json!({ "port": port }));
    }
    let listener = TcpListener::bind(("0.0.0.0", 0)).map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    if MEDIA_RUNNING.swap(true, Ordering::SeqCst) { return Ok(json!({ "port": port })); }
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let Ok(peer) = stream.peer_addr() else { continue };
            let handle = app.clone();
            std::thread::spawn(move || handle_media(handle, stream, peer));
        }
        MEDIA_RUNNING.store(false, Ordering::SeqCst);
    });
    let mut slot = MEDIA_PORT.get_or_init(|| Mutex::new(0)).lock().unwrap_or_else(|e| e.into_inner());
    *slot = port;
    Ok(json!({ "port": port }))
}

#[tauri::command]
pub fn room_media_send(
    peer: String,
    port: u16,
    session: String,
    kind: String,
    codec: String,
    sequence: u32,
    data: String,
) -> Result<(), String> {
    if session.is_empty() || session.len() > 160 || !matches!(kind.as_str(), "voice" | "watch" | "desktop") || codec.len() > 128 {
        return Err("媒体流参数无效".into());
    }
    let ip: std::net::IpAddr = peer.parse().map_err(|_| "房员地址无效".to_string())?;
    let bytes = base64::engine::general_purpose::STANDARD.decode(data).map_err(|e| e.to_string())?;
    if bytes.len() > 64 * 1024 * 1024 { return Err("媒体分片过大".into()); }
    let key = format!("{peer}:{port}:{session}:{kind}");
    let mut sinks = media_sinks().lock().unwrap_or_else(|e| e.into_inner());
    if sequence == 0 { sinks.remove(&key); }
    if !sinks.contains_key(&key) {
        let mut stream = TcpStream::connect_timeout(&SocketAddr::new(ip, port), Duration::from_secs(4))
            .map_err(|e| format!("媒体 TCP 连接失败: {e}"))?;
        stream.set_nodelay(true).map_err(|e| e.to_string())?;
        stream.set_write_timeout(Some(Duration::from_secs(30))).map_err(|e| e.to_string())?;
        stream.write_all(json!({ "session": session, "kind": kind, "codec": codec }).to_string().as_bytes()).map_err(|e| e.to_string())?;
        stream.write_all(b"\n").map_err(|e| e.to_string())?;
        sinks.insert(key.clone(), stream);
    }
    let stream = sinks.get_mut(&key).unwrap();
    let mut header = [0u8; 8];
    header[..4].copy_from_slice(&(bytes.len() as u32).to_be_bytes());
    header[4..].copy_from_slice(&sequence.to_be_bytes());
    if let Err(error) = stream.write_all(&header).and_then(|_| stream.write_all(&bytes)) {
        sinks.remove(&key);
        return Err(error.to_string());
    }
    Ok(())
}
