//! 音乐平台直连（网易云 / QQ / 酷狗）：扫码登录、搜索、播放地址、歌词。
//! 协议参考 Sorihiko/DangoMusic（MIT）的 Kotlin 实现，移植到 Rust。
//! 账号凭据只存本地（app_data/music_platform/），请求时按平台携带。

use base64::Engine as _;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36";
const NETEASE_PRESET_KEY: &[u8] = b"0CoJUm6Qyw8W8jud";
const NETEASE_IV: &[u8] = b"0102030405060708";
const NETEASE_RSA_MODULUS: &str = "00e0b509f6259df8642dbc35662901477df22677ec152b5ff68ace615bb7b725152b3ab17a876aea8a5aa76d2e417629ec4ee341f56135fccf695280104e0312ecbda92557c93870114af6c9d05c4f7f0c3685b7a46bee255932575cce10b424d813cfe4875d3e82047b97ddef52741d546b8e289dc6935b3ece0462db0a22b8e7";
const KUGOU_WEB_SIGN_KEY: &str = "NVPh5oo715z5DIWAeQlhMDsWXXQV4hwt";
const KUGOU_GATEWAY_SALT: &str = "OIlwieks28dk2k092lksi2UIkp";
const KUGOU_LOGIN_MODULUS: &str = "B1B1EC76A1BBDBF0D18E8CD9A87E53FA3881E2F004C67C9DDA2CA677DBEFA3D61DF8463FE12D84FF4B4699E02C9D41CAB917F5A8FB9E35580C4BDF97763A0420A476295D763EE10174E6F9EBF7DF8A77BA5B20CDA4EE705DEF5BBA3C88567B9656E52C9CD5CD95CA735FF2D25F762B133273EEEB7B4F3EA8B6DA29040F3B67CD";
const KUGOU_PLAY_SALT2: &str = "57ae12eb6890223e355ccfcb74edf70d";

/* ---------------- 加密 ---------------- */

fn aes_ecb_pkcs7_encrypt(plain: &[u8], key: &[u8]) -> Vec<u8> {
    use aes::cipher::generic_array::GenericArray;
    use aes::cipher::{BlockEncrypt, KeyInit};
    use aes::Aes128;
    let cipher = Aes128::new_from_slice(key).expect("AES-128 key");
    let pad = 16 - (plain.len() % 16);
    let mut padded = plain.to_vec();
    padded.extend(std::iter::repeat(pad as u8).take(pad));
    let mut out = Vec::with_capacity(padded.len());
    for chunk in padded.chunks(16) {
        let mut block = GenericArray::clone_from_slice(chunk);
        cipher.encrypt_block(&mut block);
        out.extend_from_slice(block.as_slice());
    }
    out
}

fn aes_cbc_pkcs7_encrypt(plain: &[u8], key: &[u8], iv: &[u8]) -> Vec<u8> {
    use aes::cipher::generic_array::GenericArray;
    use aes::cipher::{BlockEncrypt, KeyInit};
    use aes::Aes128;
    let cipher = Aes128::new_from_slice(key).expect("AES-128 key");
    let pad = 16 - (plain.len() % 16);
    let mut padded = plain.to_vec();
    padded.extend(std::iter::repeat(pad as u8).take(pad));
    let mut prev = iv.to_vec();
    let mut out = Vec::with_capacity(padded.len());
    for chunk in padded.chunks(16) {
        let mut block = [0u8; 16];
        for i in 0..16 {
            block[i] = chunk[i] ^ prev[i];
        }
        let mut ga = GenericArray::clone_from_slice(&block);
        cipher.encrypt_block(&mut ga);
        prev = ga.to_vec();
        out.extend_from_slice(ga.as_slice());
    }
    out
}

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    (0..hex.len() / 2)
        .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap_or(0))
        .collect()
}

fn rsa_nopad_hex(data: &[u8], modulus_hex: &str) -> String {
    use num_bigint::BigUint;
    let m = BigUint::from_bytes_be(data);
    let e = BigUint::from_bytes_be(&hex_to_bytes("010001"));
    let n = BigUint::from_bytes_be(&hex_to_bytes(modulus_hex));
    let r = m.modpow(&e, &n);
    let hex = r.to_bytes_be().iter().map(|b| format!("{:02X}", b)).collect::<String>();
    format!("{:0>256}", hex)
}

fn md5_hex_lower(value: &str) -> String {
    use md_5::{Digest, Md5};
    Md5::digest(value.as_bytes()).iter().map(|b| format!("{:02x}", b)).collect()
}

fn md5_hex_upper(value: &[u8]) -> String {
    use md_5::{Digest, Md5};
    Md5::digest(value).iter().map(|b| format!("{:02X}", b)).collect()
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(*b as char),
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/* ---------------- HTTP ---------------- */

static CLIENTS: OnceLock<Mutex<HashMap<String, reqwest::blocking::Client>>> = OnceLock::new();

fn http(platform: &str) -> &'static reqwest::blocking::Client {
    let map = CLIENTS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = map.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .entry(platform.to_string())
        .or_insert_with(|| {
            reqwest::blocking::Client::builder()
                .cookie_store(platform != "netease")
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(20))
                .build()
                .unwrap_or_else(|_| reqwest::blocking::Client::new())
        })
}

fn http_get_json(url: &str, referer: &str, cookie: &str) -> Result<Value, String> {
    let mut req = http("common")
        .get(url)
        .header("User-Agent", UA)
        .header("Accept", "application/json");
    if !referer.is_empty() {
        req = req.header("Referer", referer);
    }
    if !cookie.is_empty() {
        req = req.header("Cookie", cookie);
    }
    let resp = req.send().map_err(|e| e.to_string())?;
    let text = resp.text().map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("响应解析失败: {e}"))
}

/* ---------------- 状态与持久化 ---------------- */

#[derive(Default, Clone)]
pub struct Account {
    pub user_id: String,
    pub token: String,
    pub nickname: String,
    pub device: String,
    pub cookie: String,
}

static ACCOUNTS: OnceLock<Mutex<HashMap<String, Account>>> = OnceLock::new();

fn accounts() -> &'static Mutex<HashMap<String, Account>> {
    ACCOUNTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn state_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    use tauri::Manager;
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法定位应用数据目录: {e}"))?
        .join("music_platform");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败: {e}"))?;
    Ok(dir)
}

fn save_account(platform: &str, account: &Account, app: &tauri::AppHandle) {
    accounts()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(platform.to_string(), account.clone());
    if let Ok(dir) = state_dir(app) {
        let data = json!({
            "userId": account.user_id, "token": account.token,
            "nickname": account.nickname, "device": account.device,
            "cookie": account.cookie,
        });
        let _ = std::fs::write(dir.join(format!("{platform}.account.json")), data.to_string());
    }
}

fn load_account(platform: &str, app: &tauri::AppHandle) -> Option<Account> {
    let map = accounts().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(a) = map.get(platform) {
        return Some(a.clone());
    }
    drop(map);
    let dir = state_dir(app).ok()?;
    let text = std::fs::read_to_string(dir.join(format!("{platform}.account.json"))).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    let account = Account {
        user_id: v["userId"].as_str().unwrap_or_default().to_string(),
        token: v["token"].as_str().unwrap_or_default().to_string(),
        nickname: v["nickname"].as_str().unwrap_or_default().to_string(),
        device: v["device"].as_str().unwrap_or_default().to_string(),
        cookie: v["cookie"].as_str().unwrap_or_default().to_string(),
    };
    if account.user_id.is_empty() && account.cookie.is_empty() {
        return None;
    }
    accounts()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(platform.to_string(), account.clone());
    Some(account)
}

fn clear_account(platform: &str, app: &tauri::AppHandle) {
    accounts().lock().unwrap_or_else(|e| e.into_inner()).remove(platform);
    if let Ok(dir) = state_dir(app) {
        let _ = std::fs::remove_file(dir.join(format!("{platform}.account.json")));
    }
}

fn kugou_device(app: &tauri::AppHandle) -> String {
    if let Some(a) = load_account("kugou", app) {
        if !a.device.is_empty() {
            return a.device;
        }
    }
    // 固定设备身份：随机 32 位十六进制
    let device: String = uuid::Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(32)
        .collect();
    if let Ok(dir) = state_dir(app) {
        let _ = std::fs::write(dir.join("kugou.device"), &device);
    }
    device
}

/* ---------------- 扫码登录会话（后台线程 + 状态查询） ---------------- */

pub struct LoginSession {
    pub platform: String,
    pub state: String,
    pub qr_svg: String,
    pub qr_png: String,
    pub message: String,
}

static LOGIN: OnceLock<Mutex<Option<LoginSession>>> = OnceLock::new();
static LOGIN_SESSION: AtomicU64 = AtomicU64::new(0);

fn login() -> &'static Mutex<Option<LoginSession>> {
    LOGIN.get_or_init(|| Mutex::new(None))
}

fn set_login(platform: &str, state: &str, qr_svg: &str, qr_png: &str, message: &str) {
    *login().lock().unwrap_or_else(|e| e.into_inner()) = Some(LoginSession {
        platform: platform.to_string(),
        state: state.to_string(),
        qr_svg: qr_svg.to_string(),
        qr_png: qr_png.to_string(),
        message: message.to_string(),
    });
}

fn login_session_id() -> u64 {
    LOGIN_SESSION.load(Ordering::SeqCst)
}

fn qr_svg(content: &str) -> Result<String, String> {
    use qrcode::render::svg::Color;
    use qrcode::QrCode;
    let code = QrCode::with_error_correction_level(content.as_bytes(), qrcode::EcLevel::M)
        .map_err(|e| e.to_string())?;
    Ok(code
        .render::<Color>()
        .min_dimensions(220, 220)
        .dark_color(Color("#111111"))
        .light_color(Color("#ffffff"))
        .build())
}


/* ================= 网易云 ================= */

mod netease {
    use super::*;

    const DEFAULT_COOKIE: &str = "os=pc; appver=2.10.13";

    fn cookie(account: Option<&Account>) -> String {
        match account {
            Some(a) if !a.cookie.is_empty() => format!("{}; {DEFAULT_COOKIE}", a.cookie),
            _ => DEFAULT_COOKIE.to_string(),
        }
    }

    pub fn weapi(url: &str, body_json: &str, account: Option<&Account>) -> Result<(Value, Vec<String>), String> {
        let sec_key: String = {
            use rand::Rng;
            const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
            let mut rng = rand::thread_rng();
            (0..16)
                .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
                .collect()
        };
        let step1 = aes_cbc_pkcs7_encrypt(body_json.as_bytes(), NETEASE_PRESET_KEY, NETEASE_IV);
        let b1 = base64::engine::general_purpose::STANDARD.encode(&step1);
        let step2 = aes_cbc_pkcs7_encrypt(b1.as_bytes(), sec_key.as_bytes(), NETEASE_IV);
        let b2 = base64::engine::general_purpose::STANDARD.encode(&step2);
        let reversed: Vec<u8> = sec_key.as_bytes().iter().rev().cloned().collect();
        let enc_sec = rsa_nopad_hex(&reversed, NETEASE_RSA_MODULUS);
        let body = format!("params={}&encSecKey={}", urlencode(&b2), urlencode(&enc_sec));

        let resp = http("netease")
            .post(url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Referer", "https://music.163.com/")
            .header("User-Agent", UA)
            .header("Cookie", cookie(account))
            .body(body)
            .send()
            .map_err(|e| e.to_string())?;
        let cookies: Vec<String> = resp
            .headers()
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(String::from))
            .collect();
        let text = resp.text().map_err(|e| e.to_string())?;
        let parsed = serde_json::from_str::<Value>(&text).unwrap_or(Value::Null);
        Ok((parsed, cookies))
    }

    fn eapi_post(path: &str, payload: Value, account: Option<&Account>) -> Result<Value, String> {
        let cookie = cookie(account);
        let mut body = payload.clone();
        let mut header = json!({ "os": "pc", "appver": "2.10.13" });
        header["requestId"] = json!(format!("{}_{}", now_millis(), rand_digits(4)));
        for part in cookie.split(';') {
            let part = part.trim();
            if let Some((key, value)) = part.split_once('=') {
                if matches!(key, "MUSIC_U" | "MUSIC_A" | "__csrf") {
                    header[key] = json!(value);
                }
            }
        }
        body["header"] = header;
        let json_text = body.to_string();
        let digest = md5_hex_lower(&format!("nobody{path}use{json_text}md5forencrypt"));
        let plain = format!("{path}-36cd479b6b5-{json_text}-36cd479b6b5-{digest}");
        let enc = aes_ecb_pkcs7_encrypt(plain.as_bytes(), b"e82ckenh8dichen8")
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<String>();
        let url = format!("https://interface.music.163.com/eapi{}", path.trim_start_matches("/api"));
        let resp = http("netease")
            .post(url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("User-Agent", "Mozilla/5.0")
            .header("Referer", "https://music.163.com/")
            .header("Cookie", cookie)
            .body(format!("params={}", urlencode(&enc)))
            .send()
            .map_err(|e| e.to_string())?;
        let text = resp.text().map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| format!("响应解析失败: {e}"))
    }

    fn get_json(url: &str) -> Result<Value, String> {
        http_get_json(url, "https://music.163.com/", "")
    }

    fn get_json_auth(url: &str, account: Option<&Account>) -> Result<Value, String> {
        http_get_json(url, "https://music.163.com/", &cookie(account))
    }

    pub fn now_millis() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    pub fn rand_digits(n: usize) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..n).map(|_| format!("{}", rng.gen_range(0..10))).collect()
    }

    /* ---- 扫码登录（后台线程 + 轮询 801/802/800/803） ---- */

    pub fn qr_start(app: tauri::AppHandle) {
        std::thread::spawn(move || {
            let session = super::login_session_id();
            super::set_login("netease", "loading", "", "", "");
            let run = || -> Result<(), String> {
                let url = "https://music.163.com/weapi/login/qrcode/unikey";
                let (resp, _) = weapi(url, r#"{"type":1}"#, None)?;
                let unikey = resp["unikey"]
                    .as_str()
                    .ok_or("网易云未返回二维码 key")?
                    .to_string();
                let qr = qr_svg(&format!("https://music.163.com/login?codekey={unikey}"))?;
                super::set_login("netease", "waiting", &qr, "", "");
                loop {
                    if super::login_session_id() != session {
                        return Ok(());
                    }
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    if super::login_session_id() != session {
                        return Ok(());
                    }
                    let url = "https://music.163.com/weapi/login/qrcode/client/login";
                    let (resp, cookies) = weapi(url, &format!(r#"{{"key":"{unikey}","type":1}}"#), None)?;
                    match resp["code"].as_i64().unwrap_or(0) {
                        801 => super::set_login("netease", "waiting", &qr, "", ""),
                        802 => super::set_login("netease", "scanned", &qr, "", ""),
                        800 => {
                            super::set_login("netease", "expired", &qr, "", "");
                            return Ok(());
                        }
                        803 => {
                            let wanted = ["MUSIC_U", "__csrf", "MUSIC_A_T", "MUSIC_R_T", "NMTID"];
                            let parts: Vec<String> = cookies
                                .iter()
                                .filter_map(|sc| sc.split(';').next().map(|kv| kv.trim().to_string()))
                                .filter(|kv| {
                                    wanted.iter().any(|w| {
                                        kv.starts_with(&format!("{w}=")) && kv.len() > w.len() + 1
                                    })
                                })
                                .collect();
                            let cookie = parts.join("; ");
                            if !cookie.contains("MUSIC_U=") {
                                super::set_login("netease", "error", "", "", "登录 Cookie 不完整");
                                return Ok(());
                            }
                            let account = super::Account {
                                user_id: String::new(),
                                token: String::new(),
                                nickname: String::new(),
                                device: String::new(),
                                cookie,
                            };
                            super::save_account("netease", &account, &app);
                            super::set_login("netease", "success", &qr, "", "");
                            return Ok(());
                        }
                        _ => {}
                    }
                }
            };
            if let Err(e) = run() {
                super::set_login("netease", "error", "", "", &e);
            }
        });
    }

    pub fn account(app: &tauri::AppHandle) -> Result<Value, String> {
        let account = super::load_account("netease", app);
        let logged_in = account
            .as_ref()
            .map(|a| a.cookie.contains("MUSIC_U="))
            .unwrap_or(false);
        if !logged_in {
            return Ok(json!({ "loggedIn": false }));
        }
        let o = get_json_auth("https://music.163.com/api/nuser/account/get", account.as_ref())?;
        Ok(json!({
            "loggedIn": true,
            "nickname": o["profile"]["nickname"].as_str().unwrap_or_default(),
            "avatar": o["profile"]["avatarUrl"].as_str().unwrap_or_default(),
        }))
    }

    /* ---- 歌单 ---- */

    pub fn playlists(app: &tauri::AppHandle) -> Result<Value, String> {
        let account = super::load_account("netease", app);
        let uid = account
            .as_ref()
            .and_then(|a| {
                let o = get_json_auth("https://music.163.com/api/nuser/account/get", Some(a)).ok()?;
                o["account"]["id"].as_i64()
            })
            .ok_or("请先扫码登录网易云")?;
        let o = get_json_auth(
            &format!("https://music.163.com/api/user/playlist/?uid={uid}&limit=1000&offset=0"),
            account.as_ref(),
        )?;
        let mut out = Vec::new();
        if let Some(list) = o["playlist"].as_array() {
            for p in list {
                let id = p["id"].as_i64().unwrap_or(0);
                if id == 0 {
                    continue;
                }
                out.push(json!({
                    "id": id.to_string(),
                    "name": p["name"].as_str().unwrap_or_default(),
                    "count": p["trackCount"].as_i64().unwrap_or(0),
                    "cover": p["coverImgUrl"].as_str().unwrap_or_default(),
                    "owned": p["creator"]["userId"].as_i64() == Some(uid),
                }));
            }
        }
        Ok(json!({ "playlists": out }))
    }

    fn parse_track(v: &Value) -> Option<Value> {
        let id = v["id"].as_i64()?;
        let artists: Vec<String> = v["artists"]
            .as_array()
            .or_else(|| v["ar"].as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x["name"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        Some(json!({
            "key": format!("netease:{id}"),
            "platform": "netease",
            "id": id.to_string(),
            "title": v["name"].as_str().unwrap_or_default(),
            "artists": artists,
            "durationMs": v["duration"].as_i64().or_else(|| v["dt"].as_i64()).unwrap_or(0),
            "cover": v["album"]["picUrl"].as_str()
                .or_else(|| v["al"]["picUrl"].as_str())
                .unwrap_or_default(),
        }))
    }

    pub fn playlist_tracks(app: &tauri::AppHandle, id: &str) -> Result<Value, String> {
        let account = super::load_account("netease", app);
        let o = get_json_auth(
            &format!("https://music.163.com/api/v6/playlist/detail?id={id}&n=200&s=0"),
            account.as_ref(),
        )?;
        let mut known: Vec<Value> = Vec::new();
        let mut by_id: std::collections::HashMap<i64, Value> = std::collections::HashMap::new();
        if let Some(tracks) = o["playlist"]["tracks"].as_array() {
            for t in tracks {
                if let Some(tv) = parse_track(t) {
                    if let Some(id) = tv["id"].as_str().and_then(|s| s.parse::<i64>().ok()) {
                        by_id.insert(id, tv.clone());
                    }
                    known.push(tv);
                }
            }
        }
        let ids: Vec<i64> = o["playlist"]["trackIds"]
            .as_array()
            .map(|a| a.iter().filter_map(|x| x["id"].as_i64()).collect())
            .unwrap_or_default();
        let missing: Vec<i64> = ids.iter().filter(|id| !by_id.contains_key(id)).cloned().collect();
        for batch in missing.chunks(200) {
            let id_list = format!("[{}]", batch.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(","));
            if let Ok(detail) = get_json_auth(
                &format!("https://music.163.com/api/song/detail?ids={}", urlencode(&id_list)),
                account.as_ref(),
            ) {
                if let Some(songs) = detail["songs"].as_array() {
                    for t in songs {
                        if let Some(tv) = parse_track(t) {
                            if let Some(id) = tv["id"].as_str().and_then(|s| s.parse::<i64>().ok()) {
                                by_id.entry(id).or_insert(tv.clone());
                            }
                        }
                    }
                }
            }
        }
        let ordered: Vec<Value> = ids
            .iter()
            .filter_map(|id| by_id.get(id).cloned())
            .collect();
        Ok(json!({ "tracks": ordered }))
    }

    /* ---- 搜索（三级降级：cloudsearch → weapi → 老接口） ---- */

    pub fn search(query: &str) -> Result<Value, String> {
        let q = query.trim();
        let enc = urlencode(q);
        if let Ok(o) = get_json(&format!(
            "https://music.163.com/api/cloudsearch/pc?s={enc}&type=1&offset=0&limit=40"
        )) {
            if let Some(songs) = o["result"]["songs"].as_array() {
                let tracks: Vec<Value> = songs.iter().filter_map(parse_track).collect();
                if !tracks.is_empty() {
                    return Ok(json!({ "tracks": tracks }));
                }
            }
        }
        let body = json!({
            "s": q, "type": "1", "limit": "40", "offset": "0", "total": "true", "csrf_token": ""
        });
        if let Ok((resp, _)) = weapi(
            "https://music.163.com/weapi/cloudsearch/get/web",
            &body.to_string(),
            None,
        ) {
            if let Some(songs) = resp["result"]["songs"].as_array() {
                let tracks: Vec<Value> = songs.iter().filter_map(parse_track).collect();
                if !tracks.is_empty() {
                    return Ok(json!({ "tracks": tracks }));
                }
            }
        }
        let o = get_json(&format!(
            "https://music.163.com/api/search/get/web?s={enc}&type=1&offset=0&limit=40"
        ))?;
        let basic: Vec<Value> = o["result"]["songs"]
            .as_array()
            .map(|a| a.iter().filter_map(parse_track).collect())
            .unwrap_or_default();
        if basic.is_empty() {
            return Ok(json!({ "tracks": [] }));
        }
        let ids = format!(
            "[{}]",
            basic
                .iter()
                .filter_map(|t| t["id"].as_str().and_then(|s| s.parse::<i64>().ok()))
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        let mut detailed: std::collections::HashMap<String, Value> = std::collections::HashMap::new();
        if let Ok(detail) = get_json(&format!(
            "https://music.163.com/api/song/detail?ids={}",
            urlencode(&ids)
        )) {
            if let Some(songs) = detail["songs"].as_array() {
                for t in songs {
                    if let Some(tv) = parse_track(t) {
                        detailed.insert(tv["key"].as_str().unwrap_or_default().to_string(), tv);
                    }
                }
            }
        }
        let tracks: Vec<Value> = basic
            .into_iter()
            .map(|t| {
                detailed
                    .get(t["key"].as_str().unwrap_or_default())
                    .filter(|d| d["cover"].as_str().map(|c| !c.is_empty()).unwrap_or(false))
                    .cloned()
                    .unwrap_or(t)
            })
            .collect();
        Ok(json!({ "tracks": tracks }))
    }

    /* ---- 播放地址（登录走 eapi v1，未登录走旧码率接口，最后外链兜底） ---- */

    pub fn song_url(app: &tauri::AppHandle, id: &str) -> Result<Value, String> {
        let id_num: i64 = id.parse().map_err(|_| "无效的歌曲 ID".to_string())?;
        let account = super::load_account("netease", app);
        let mut payload = json!({ "ids": json!(format!("[{id_num}]")) });
        if account.is_some() {
            payload["level"] = json!("exhigh");
            payload["encodeType"] = json!("mp3");
        } else {
            payload["br"] = json!(192000);
        }
        let attempts: Vec<(&str, String)> = vec![
            ("v1", payload.to_string()),
            (
                "legacy",
                json!({ "ids": json!(format!("[{id_num}]")), "br": 192000 }).to_string(),
            ),
        ];
        for (stage, body) in attempts {
            let root = if stage == "v1" {
                netease::eapi_post(
                    "/api/song/enhance/player/url/v1",
                    serde_json::from_str(&body).unwrap_or(Value::Null),
                    account.as_ref(),
                )
            } else {
                get_json_auth(
                    &format!("https://music.163.com/api/song/enhance/player/url?ids=[{id_num}]&br=192000"),
                    account.as_ref(),
                )
            };
            let Ok(root) = root else { continue };
            if root["code"].as_i64() != Some(200) {
                continue;
            }
            if let Some(row) = root["data"].as_array().and_then(|a| a.first()) {
                let url = row["url"].as_str().unwrap_or_default();
                if url.is_empty() {
                    continue;
                }
                let kind = row["type"].as_str().unwrap_or_default().to_lowercase();
                if !kind.is_empty() && !matches!(kind.as_str(), "mp3" | "ogg" | "vorbis" | "flac") {
                    continue;
                }
                return Ok(json!({
                    "url": url,
                    "trial": row["freeTrialInfo"].is_object(),
                    "format": if kind.is_empty() { "mp3".to_string() } else { kind },
                }));
            }
        }
        Ok(json!({
            "url": format!("https://music.163.com/song/media/outer/url?id={id_num}.mp3"),
            "trial": false,
            "format": "mp3",
        }))
    }

    /* ---- 歌词（公开接口 + 翻译合并 + credit 行过滤） ---- */

    pub fn lyrics(id: &str) -> Result<Value, String> {
        let o = get_json(&format!(
            "https://music.163.com/api/song/lyric?id={id}&lv=1&tv=-1"
        ))?;
        let raw = o["lrc"]["lyric"].as_str().unwrap_or_default();
        let translated = o["tlyric"]["lyric"].as_str().unwrap_or_default();
        let lines = super::lyrics_util::parse_lrc(raw);
        if lines.is_empty() {
            return Ok(json!({ "found": false, "lines": [] }));
        }
        let trans = super::lyrics_util::parse_lrc(translated);
        let merged = super::lyrics_util::merge_translation(&lines, &trans);
        Ok(json!({ "found": true, "lines": merged }))
    }
}


/* ================= QQ 音乐（免登录：搜索 / 歌词 / 免费歌曲播放） ================= */

mod qq {
    use super::*;
    const REFERER: &str = "https://y.qq.com/";

    pub fn search(query: &str) -> Result<Value, String> {
        let url = format!(
            "https://c.y.qq.com/soso/fcgi-bin/client_search_cp?format=json&p=1&n=40&w={}",
            urlencode(query.trim())
        );
        let o = http_get_json(&url, REFERER, "")?;
        if o["code"].as_i64() != Some(0) {
            return Err("QQ 音乐搜索被拒绝".into());
        }
        let mut out = Vec::new();
        if let Some(list) = o["data"]["song"]["list"].as_array() {
            for s in list {
                let mid = s["songmid"]
                    .as_str()
                    .or_else(|| s["mid"].as_str())
                    .unwrap_or_default()
                    .to_string();
                if mid.is_empty() || mid.len() > 64 || !mid.chars().all(|c| c.is_ascii_alphanumeric()) {
                    continue;
                }
                let media_mid = s["strMediaMid"]
                    .as_str()
                    .or_else(|| s["media_mid"].as_str())
                    .or_else(|| s["file"]["media_mid"].as_str())
                    .unwrap_or_default()
                    .to_string();
                let artists: Vec<String> = s["singer"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x["name"].as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let album_mid = s["albummid"]
                    .as_str()
                    .or_else(|| s["album"]["mid"].as_str())
                    .unwrap_or_default()
                    .to_string();
                out.push(json!({
                    "key": format!("qq:{mid}"),
                    "platform": "qq",
                    "id": mid,
                    "mediaMid": media_mid,
                    "title": s["songname"].as_str().unwrap_or_default(),
                    "artists": artists,
                    "durationMs": s["interval"].as_i64().unwrap_or(0) * 1000,
                    "cover": if album_mid.is_empty() { String::new() } else {
                        format!("https://y.gtimg.cn/music/photo_new/T002R300x300M000{album_mid}.jpg")
                    },
                }));
            }
        }
        registry_put(&out);
        Ok(json!({ "tracks": out }))
    }

    pub fn lyrics(mid: &str) -> Result<Value, String> {
        // 纯 LRC 行级歌词（公开接口）；QRC 逐字歌词需要专用 DES 解码，暂不启用
        let url = format!(
            "https://c.y.qq.com/lyric/fcgi-bin/fcg_query_lyric_new.fcg?format=json&nobase64=1&songmid={}",
            urlencode(mid)
        );
        let o = http_get_json(&url, REFERER, "")?;
        let raw = o["lyric"].as_str().unwrap_or_default();
        let lines = super::lyrics_util::parse_lrc(raw);
        Ok(json!({ "found": !lines.is_empty(), "lines": lines }))
    }

    pub fn song_url(media_mid: &str, song_mid: &str) -> Result<Value, String> {
        let guid: String = rand_digits(10);
        let media = if media_mid.is_empty() { song_mid.to_string() } else { media_mid.to_string() };
        let payload = json!({
            "comm": { "ct": 24, "cv": 0 },
            "req_0": {
                "module": "vkey.GetVkeyServer",
                "method": "CgiGetVkey",
                "param": {
                    "guid": guid,
                    "uin": "0",
                    "filename": [format!("M500{media}.mp3")],
                    "songmid": [song_mid],
                    "songtype": [0],
                }
            }
        });
        let resp = http("qq")
            .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
            .header("Content-Type", "application/json")
            .header("Referer", REFERER)
            .header("User-Agent", UA)
            .body(payload.to_string())
            .send()
            .map_err(|e| e.to_string())?;
        let root: Value = resp.json().map_err(|e| e.to_string())?;
        if root["code"].as_i64() != Some(0) {
            return Err("QQ 音乐播放请求被拒绝".into());
        }
        let hosts: Vec<String> = root["data"]["sip"]
            .as_array()
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();
        for item in root["data"]["midurlinfo"].as_array().unwrap_or(&Vec::new()) {
            let result = item["result"].as_i64().unwrap_or(0);
            if result != 0 {
                continue;
            }
            let purl = item["purl"].as_str().unwrap_or_default();
            if purl.is_empty() {
                continue;
            }
            let mut candidates: Vec<String> = if purl.starts_with("https://") || purl.starts_with("http://") {
                vec![purl.to_string()]
            } else {
                let mut list: Vec<String> = hosts
                    .iter()
                    .map(|h| format!("{}{}", h.trim_end_matches('/'), purl))
                    .collect();
                list.push(format!("https://isure.stream.qqmusic.qq.com/{purl}"));
                list
            };
            candidates.retain(|c| c.ends_with(".mp3") || c.ends_with(".flac"));
            for candidate in candidates {
                return Ok(json!({ "url": candidate, "trial": false, "format": "mp3" }));
            }
        }
        Err("QQ 音乐没有返回可播放的音频（会员歌曲需要登录，登录功能后续开放）".into())
    }

}

/* ================= 酷狗（扫码登录 + 搜索 / 播放 / 歌词） ================= */

mod kugou {
    use super::*;

    fn signed(values: &Value, device: &str) -> Value {
        let mut params = json!({
            "appid": "1014", "srcappid": "2919", "clientver": "20000",
            "clienttime": now_millis().to_string(), "mid": device, "uuid": device, "dfid": "-",
        });
        if let (Some(base), Some(extra)) = (params.as_object_mut(), values.as_object()) {
            for (k, v) in extra {
                base.insert(k.clone(), v.clone());
            }
        }
        // signature = md5(KEY + sorted(k=v) + KEY)
        let mut obj = params.as_object().unwrap().clone();
        let mut pairs: Vec<(String, String)> = obj
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().unwrap_or(&v.to_string()).to_string()))
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        let material = format!(
            "{KUGOU_WEB_SIGN_KEY}{}{KUGOU_WEB_SIGN_KEY}",
            pairs.iter().map(|(k, v)| format!("{k}={v}")).collect::<String>()
        );
        obj.insert("signature".into(), json!(md5_hex_upper(material.as_bytes())));
        Value::Object(obj)
    }

    fn kugou_get(url: &str, values: &Value, device: &str, cookies: bool) -> Result<Value, String> {
        let signed = signed(values, device);
        let obj = signed.as_object().unwrap();
        let query: Vec<String> = obj
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencode(v.as_str().unwrap_or(&v.to_string()))))
            .collect();
        let mut req = http("kugou").get(format!("{url}?{}", query.join("&")));
        if cookies {
            req = http("kugou").get(format!("{url}?{}", query.join("&")));
        }
        let resp = req
            .header("User-Agent", UA)
            .send()
            .map_err(|e| e.to_string())?;
        let text = resp.text().map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| format!("响应解析失败: {e}"))
    }

    pub fn qr_start(app: tauri::AppHandle) {
        std::thread::spawn(move || {
            let session = super::login_session_id();
            super::set_login("kugou", "loading", "", "", "");
            let device = kugou_device(&app);
            let run = || -> Result<(), String> {
                let values = json!({
                    "clientver": "1000", "type": "1", "plat": "4",
                    "qrcode_txt": "https://h5.kugou.com/apps/loginQRCode/html/index.html?appid=1014&",
                });
                let root = kugou_get(
                    "https://login-user.kugou.com/v2/qrcode",
                    &values,
                    &device,
                    true,
                )?;
                if root["status"].as_i64() != Some(1) {
                    return Err("酷狗二维码获取被拒绝".into());
                }
                let key = root["data"]["qrcode"]
                    .as_str()
                    .ok_or("酷狗未返回二维码 key")?
                    .to_string();
                let png = root["data"]["qrcode_img"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                if png.starts_with("data:image/") {
                    super::set_login("kugou", "waiting", "", &png, "");
                } else {
                    let qr = qr_svg(&format!(
                        "https://h5.kugou.com/apps/loginQRCode/html/index.html?appid=1014&qrcode={key}"
                    ))?;
                    super::set_login("kugou", "waiting", &qr, "", "");
                }
                let started = std::time::Instant::now();
                loop {
                    if super::login_session_id() != session {
                        return Ok(());
                    }
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    if super::login_session_id() != session {
                        return Ok(());
                    }
                    if started.elapsed() > std::time::Duration::from_secs(180) {
                        super::set_login("kugou", "expired", "", "", "");
                        return Ok(());
                    }
                    let values = json!({ "clientver": "1000", "plat": "4", "qrcode": key });
                    let poll = kugou_get(
                        "https://login-user.kugou.com/v2/get_userinfo_qrcode",
                        &values,
                        &device,
                        true,
                    )?;
                    if poll["status"].as_i64() != Some(1) {
                        continue;
                    }
                    match poll["data"]["status"].as_i64().unwrap_or(1) {
                        1 => {}
                        2 => super::set_login("kugou", "scanned", "", "", ""),
                        4 => {
                            let info = poll["data"]["userinfo"]
                                .as_object()
                                .or_else(|| poll["data"]["user_info"].as_object())
                                .unwrap_or(poll["data"].as_object().unwrap_or(&json!({})));
                            let user_id = info["userid"]
                                .as_str()
                                .or_else(|| poll["data"]["userid"].as_str())
                                .unwrap_or_default()
                                .to_string();
                            let token = info["token"]
                                .as_str()
                                .or_else(|| poll["data"]["token"].as_str())
                                .unwrap_or_default()
                                .to_string();
                            if user_id.is_empty() || token.is_empty() {
                                return Err("酷狗扫码凭据不完整".into());
                            }
                            let nickname = info["nickname"].as_str().unwrap_or_default().to_string();
                            // 扫码 token 一次性：换成网页登录会话
                            let exchanged = token_login(&user_id, &token, &device)?;
                            let web_token = exchanged["data"]["token"]
                                .as_str()
                                .unwrap_or(&token)
                                .to_string();
                            let account = super::Account {
                                user_id,
                                token: web_token,
                                nickname,
                                device,
                                cookie: String::new(),
                            };
                            super::save_account("kugou", &account, &app);
                            super::set_login("kugou", "success", "", "", "");
                            return Ok(());
                        }
                        _ => {
                            super::set_login("kugou", "expired", "", "", "");
                            return Ok(());
                        }
                    }
                }
            };
            if let Err(e) = run() {
                super::set_login("kugou", "error", "", "", &e);
            }
        });
    }

    fn token_login(user_id: &str, qr_token: &str, device: &str) -> Result<Value, String> {
        let now = now_millis();
        let key: String = {
            use rand::Rng;
            const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
            let mut rng = rand::thread_rng();
            (0..16)
                .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
                .collect()
        };
        // encryptToken：AES-CBC-PKCS5(key = md5(key16)，iv = 摘要末 16 字节) + RSA 128 字节块
        use md_5::{Digest, Md5};
        let digest = Md5::digest(key.as_bytes());
        let hex_digest: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
        let key_bytes: Vec<u8> = hex_digest.as_bytes()[..16].to_vec();
        let iv_bytes: Vec<u8> = hex_digest.as_bytes()[16..32].to_vec();
        let payload = json!({ "token": qr_token }).to_string();
        let encrypted = aes_cbc_pkcs7_encrypt(payload.as_bytes(), &key_bytes, &iv_bytes)
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
        let metadata = json!({ "clienttime_ms": now, "key": key }).to_string();
        let mut block = vec![0u8; 128];
        block[..metadata.len()].copy_from_slice(metadata.as_bytes());
        let encoded = rsa_nopad_hex_128(&block, KUGOU_LOGIN_MODULUS);
        let mut params = json!({
            "clientver": "20000", "clienttime": (now / 1000).to_string(), "dev": "web",
            "userid": user_id, "plat": "4", "clienttime_ms": now.to_string(),
            "pk": encoded, "params": encrypted,
        });
        // 附加签名字段
        let mut obj = params.as_object().unwrap().clone();
        let mut pairs: Vec<(String, String)> = obj
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().unwrap_or(&v.to_string()).to_string()))
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        let material = format!(
            "{KUGOU_WEB_SIGN_KEY}{}{KUGOU_WEB_SIGN_KEY}",
            pairs.iter().map(|(k, v)| format!("{k}={v}")).collect::<String>()
        );
        obj.insert("signature".into(), json!(md5_hex_upper(material.as_bytes())));
        params = Value::Object(obj);
        let obj = params.as_object().unwrap();
        let query: Vec<String> = obj
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencode(v.as_str().unwrap_or(&v.to_string()))))
            .collect();
        let url = format!(
            "https://loginservice.kugou.com/v1/login_by_token_get?{}",
            query.join("&")
        );
        let resp = http("kugou")
            .get(&url)
            .header("User-Agent", UA)
            .send()
            .map_err(|e| e.to_string())?;
        let text = resp.text().map_err(|e| e.to_string())?;
        let root: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        if root["status"].as_i64() != Some(1) {
            return Err(format!(
                "酷狗拒绝了登录换票（error {}）",
                root["error_code"].as_i64().unwrap_or(0)
            ));
        }
        Ok(root)
    }

    fn rsa_nopad_hex_128(block: &[u8], modulus_hex: &str) -> String {
        use num_bigint::BigUint;
        let m = BigUint::from_bytes_be(block);
        let e = BigUint::from_bytes_be(&[1, 0, 1]);
        let n = BigUint::from_bytes_be(&hex_to_bytes(modulus_hex));
        let r = m.modpow(&e, &n);
        let mut hex = r.to_bytes_be().iter().map(|b| format!("{:02x}", b)).collect::<String>();
        while hex.len() % 4 != 0 {
            hex.insert(0, '0');
        }
        hex
    }

    pub fn account(app: &tauri::AppHandle) -> Result<Value, String> {
        let account = super::load_account("kugou", app);
        match account {
            Some(a) if !a.token.is_empty() => Ok(json!({
                "loggedIn": true,
                "nickname": a.nickname,
            })),
            _ => Ok(json!({ "loggedIn": false })),
        }
    }

    pub fn search(query: &str) -> Result<Value, String> {
        let url = format!(
            "https://msearchretry.kugou.com/api/v3/search/song?format=json&page=1&pagesize=40&showtype=1&keyword={}",
            urlencode(query.trim())
        );
        let o = http_get_json(&url, "https://m.kugou.com/", "")?;
        if o["status"].as_i64() != Some(1) {
            return Err("酷狗搜索被拒绝".into());
        }
        let mut out = Vec::new();
        if let Some(list) = o["data"]["info"].as_array() {
            for s in list {
                let hash = s["hash"]
                    .as_str()
                    .or_else(|| s["Hash"].as_str())
                    .unwrap_or_default()
                    .to_uppercase();
                if hash.len() != 32 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
                    continue;
                }
                let artists: Vec<String> = s["singerinfo"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x["name"].as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let album_id = s["album_id"]
                    .as_str()
                    .or_else(|| s["albumInfo"]["album_id"].as_str())
                    .unwrap_or_default()
                    .to_string();
                let ms = s["timelen"].as_i64().unwrap_or(0);
                out.push(json!({
                    "key": format!("kugou:{hash}"),
                    "platform": "kugou",
                    "id": hash,
                    "title": s["songname"].as_str().unwrap_or_default(),
                    "artists": artists,
                    "durationMs": ms,
                    "cover": s["imgurl"].as_str().unwrap_or_default().replace("{size}", "300"),
                    "albumId": album_id,
                    "albumAudioId": s["album_audio_id"].as_str().unwrap_or_default(),
                }));
            }
        }
        registry_put(&out);
        Ok(json!({ "tracks": out }))
    }

    pub fn song_url(_app: &tauri::AppHandle, track: &Value) -> Result<Value, String> {
        let hash = track["id"].as_str().unwrap_or_default().to_lowercase();
        // 游客接口（免费歌曲可放；会员歌曲只给试听或失败）
        let info_url = format!(
            "https://m.kugou.com/app/i/getSongInfo.php?cmd=playInfo&hash={}",
            urlencode(&hash)
        );
        let o = http_get_json(&info_url, "https://m.kugou.com/", "")?;
        if o["status"].as_i64() == Some(1) {
            let data = o.get("data").cloned().unwrap_or(o.clone());
            for key in ("play_url", "url", "backup_url") {
                if let Some(url) = data[key].as_str().filter(|u| u.starts_with("http")) {
                    for part in url.split(",http") {
                        let candidate = if part.starts_with("http") {
                            part.to_string()
                        } else {
                            format!("http{part}")
                        };
                        let ext_ok = candidate.ends_with(".mp3")
                            || candidate.ends_with(".flac")
                            || candidate.contains(".mp3?")
                            || candidate.contains(".mp3&");
                        if ext_ok {
                            return Ok(json!({ "url": candidate, "trial": false, "format": "mp3" }));
                        }
                    }
                }
            }
        }
        Err("酷狗没有返回可播放的音频（会员歌曲需要登录，登录功能后续开放）".into())
    }

    pub fn lyrics(hash: &str, duration_ms: i64) -> Result<Value, String> {
        let search_url = format!(
            "https://lyrics.kugou.com/search?ver=1&man=yes&client=pc&hash={}&duration={}",
            urlencode(hash),
            duration_ms
        );
        let search = http_get_json(&search_url, "https://m.kugou.com/", "")?;
        let candidate = search["candidates"]
            .as_array()
            .and_then(|a| a.first())
            .ok_or("没有匹配的歌词")?;
        let id = candidate["id"].as_i64().unwrap_or(0);
        let access_key = candidate["accesskey"].as_str().unwrap_or_default();
        let download_url = format!(
            "https://lyrics.kugou.com/download?ver=1&client=pc&fmt=lrc&charset=utf8&id={}&accesskey={}",
            id,
            urlencode(access_key)
        );
        let download = http_get_json(&download_url, "https://m.kugou.com/", "")?;
        let content = download["content"].as_str().unwrap_or_default();
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(content)
            .map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&decoded).to_string();
        let lines = super::lyrics_util::parse_lrc(&text);
        Ok(json!({ "found": !lines.is_empty(), "lines": lines }))
    }
}

/* ================= 歌词工具（LRC 解析 + credit 行过滤 + 翻译合并） ================= */

pub mod lyrics_util {
    use super::*;

    fn is_credit(line: &str) -> bool {
        const WORDS: &[&str] = &[
            "作词", "作曲", "编曲", "制作人", "制作", "混音", "母带", "和声", "吉他", "贝斯",
            "鼓", "弦乐", "录音", "监制", "出品", "发行", "统筹", "词", "曲", "OP", "SP",
            "Lyricist", "Lyrics by", "Composer", "Composed by", "Producer", "Produced by",
            "Arranger", "Arranged by",
        ];
        let trimmed = line.trim();
        WORDS.iter().any(|w| {
            trimmed
                .strip_prefix(w)
                .map(|rest| rest.starts_with(':') || rest.starts_with('：') || rest.starts_with(" :") || rest.starts_with(" ："))
                .unwrap_or(false)
        })
    }

    fn time_tag(tag: &str) -> Option<i64> {
        let (m, s) = tag.split_once(':')?;
        let m: i64 = m.trim().parse().ok()?;
        let s: f64 = s.trim().parse().ok()?;
        if m < 0 || s < 0 {
            return None;
        }
        Some(m * 60_000 + (s * 1000.0) as i64)
    }

    pub fn parse_lrc(text: &str) -> Vec<Value> {
        let tag = regex::Regex::new(r"^\[(\d{1,3}):(\d{1,2})(?:[.:](\d{1,3}))?\]").unwrap();
        let mut out = Vec::new();
        for raw in text.lines() {
            let mut tags: Vec<i64> = Vec::new();
            let mut rest = raw.trim();
            loop {
                let Some(m) = tag.captures(rest) else { break };
                if m.get(0).unwrap().start() != 0 {
                    break;
                }
                let min: i64 = m.get(1).unwrap().as_str().parse().unwrap_or(0);
                let sec: i64 = m.get(2).unwrap().as_str().parse().unwrap_or(0);
                let frac_raw = m.get(3).map(|x| x.as_str()).unwrap_or("");
                let frac: i64 = match frac_raw.len() {
                    0 => 0,
                    1 => frac_raw.parse::<i64>().unwrap_or(0) * 100,
                    2 => frac_raw.parse::<i64>().unwrap_or(0) * 10,
                    _ => frac_raw[..3].parse::<i64>().unwrap_or(0),
                };
                tags.push(min * 60_000 + sec * 1000 + frac);
                rest = &rest[m.get(0).unwrap().end()..];
            }
            if tags.is_empty() {
                continue;
            }
            let lyric = rest.trim();
            if lyric.is_empty() || is_credit(lyric) {
                continue;
            }
            for t in tags {
                out.push(json!({ "t": t, "l": lyric }));
            }
        }
        out.sort_by_key(|x| x["t"].as_i64().unwrap_or(0));
        out
    }

    /// 把翻译按时间对齐进主歌词（{l: 主, tr: 翻译}）
    pub fn merge_translation(main: &[Value], translation: &[Value]) -> Vec<Value> {
        if translation.is_empty() {
            return main.to_vec();
        }
        main.iter()
            .map(|line| {
                let t = line["t"].as_i64().unwrap_or(0);
                let nearest = translation
                    .iter()
                    .min_by_key(|x| (x["t"].as_i64().unwrap_or(0) - t).abs());
                let mut out = line.clone();
                if let Some(tr) = nearest {
                    if (tr["t"].as_i64().unwrap_or(0) - t).abs() < 3000 {
                        out["tr"] = json!(tr["l"].as_str().unwrap_or_default());
                    }
                }
                out
            })
            .collect()
    }
}

/* ================= 曲目注册表（key → 详情，播放地址解析用） ================= */

static TRACKS: OnceLock<Mutex<HashMap<String, Value>>> = OnceLock::new();

fn tracks() -> &'static Mutex<HashMap<String, Value>> {
    TRACKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn registry_put(tracks: &[Value]) {
    let mut map = tracks().lock().unwrap_or_else(|e| e.into_inner());
    for t in tracks {
        if let Some(key) = t["key"].as_str() {
            map.insert(key.to_string(), t.clone());
        }
    }
    if map.len() > 500 {
        let keys: Vec<String> = map.keys().cloned().collect();
        for key in keys[..keys.len() - 300].iter() {
            map.remove(key);
        }
    }
}

fn registry_get(key: &str) -> Option<Value> {
    tracks()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(key)
        .cloned()
}

/* ================= Tauri 命令 ================= */

fn platform_supported(platform: &str) -> bool {
    matches!(platform, "netease" | "qq" | "kugou")
}

#[tauri::command]
fn music_login_start(platform: String, app: tauri::AppHandle) -> Result<(), String> {
    if !platform_supported(&platform) {
        return Err(format!("{platform} 暂不支持"));
    }
    if platform == "qq" {
        return Err("QQ 扫码登录暂未开放，当前可直接搜索播放免费歌曲".into());
    }
    super_login_session_bump();
    match platform.as_str() {
        "netease" => music_platform::netease::qr_start(app),
        "kugou" => music_platform::kugou::qr_start(app),
        _ => Err("不支持的平台".into()),
    }
    Ok(())
}

fn super_login_session_bump() {
    LOGIN_SESSION.fetch_add(1, Ordering::SeqCst);
}

#[tauri::command]
fn music_login_state() -> Value {
    let guard = login().lock().unwrap_or_else(|e| e.into_inner());
    match guard.as_ref() {
        Some(s) => json!({
            "platform": s.platform, "state": s.state,
            "qrSvg": s.qr_svg, "qrPng": s.qr_png, "message": s.message,
        }),
        None => json!({ "state": "idle" }),
    }
}

#[tauri::command]
fn music_status(platform: String, app: tauri::AppHandle) -> Result<Value, String> {
    match platform.as_str() {
        "netease" => music_platform::netease::account(&app),
        "kugou" => music_platform::kugou::account(&app),
        "qq" => Ok(json!({ "loggedIn": false, "note": "QQ 登录暂未开放，可直接搜索播放免费歌曲" })),
        _ => Err("不支持的平台".into()),
    }
}

#[tauri::command]
fn music_logout(platform: String, app: tauri::AppHandle) -> Result<(), String> {
    clear_account(&platform, &app);
    Ok(())
}

#[tauri::command]
fn music_playlists(platform: String, app: tauri::AppHandle) -> Result<Value, String> {
    match platform.as_str() {
        "netease" => music_platform::netease::playlists(&app),
        "kugou" => Err("酷狗歌单浏览暂未开放".into()),
        _ => Err("不支持的平台".into()),
    }
}

#[tauri::command]
fn music_playlist_tracks(platform: String, id: String, app: tauri::AppHandle) -> Result<Value, String> {
    match platform.as_str() {
        "netease" => music_platform::netease::playlist_tracks(&app, &id),
        _ => Err("不支持的平台".into()),
    }
}

#[tauri::command]
fn music_search(platform: String, query: String) -> Result<Value, String> {
    match platform.as_str() {
        "netease" => music_platform::netease::search(&query),
        "qq" => music_platform::qq::search(&query),
        "kugou" => music_platform::kugou::search(&query),
        _ => Err("不支持的平台".into()),
    }
}

#[tauri::command]
fn music_play_url(platform: String, key: String, app: tauri::AppHandle) -> Result<Value, String> {
    let track = registry_get(&key).ok_or("曲目信息已过期，请重新搜索")?;
    match platform.as_str() {
        "netease" => {
            let id = track["id"].as_str().unwrap_or_default().to_string();
            music_platform::netease::song_url(&app, &id)
        }
        "qq" => {
            let media = track["mediaMid"].as_str().unwrap_or_default().to_string();
            let mid = track["id"].as_str().unwrap_or_default().to_string();
            music_platform::qq::song_url(&media, &mid)
        }
        "kugou" => music_platform::kugou::song_url(&app, &track),
        _ => Err("不支持的平台".into()),
    }
}

#[tauri::command]
fn music_lyrics(platform: String, key: String) -> Result<Value, String> {
    let track = registry_get(&key).ok_or("曲目信息已过期，请重新搜索")?;
    match platform.as_str() {
        "netease" => {
            let id = track["id"].as_str().unwrap_or_default().to_string();
            music_platform::netease::lyrics(&id)
        }
        "qq" => {
            let mid = track["id"].as_str().unwrap_or_default().to_string();
            music_platform::qq::lyrics(&mid)
        }
        "kugou" => {
            let hash = track["id"].as_str().unwrap_or_default().to_string();
            let ms = track["durationMs"].as_i64().unwrap_or(0);
            music_platform::kugou::lyrics(&hash, ms)
        }
        _ => Err("不支持的平台".into()),
    }
}
