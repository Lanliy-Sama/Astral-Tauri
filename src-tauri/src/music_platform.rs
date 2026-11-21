//! 音乐平台直连（网易云 / QQ / 酷狗）：扫码登录、搜索、播放地址、歌词。
//! 协议参考 Sorihiko/DangoMusic（MIT）的 Kotlin 实现，移植到 Rust。
//! 账号凭据只存本地（app_data/music_platform/），请求时按平台携带。

use base64::Engine as _;
// 自引用：命令层以 `music_platform::netease::…` 等全路径调用子模块
use crate::music_platform;
// QQ 扫码 MQTT 通道：native-tls（Windows 走 SChannel）+ sha1（WebSocket Accept 校验）
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
    use md5::{Digest, Md5};
    Md5::digest(value.as_bytes()).iter().map(|b| format!("{:02x}", b)).collect()
}

fn md5_hex_upper(value: &[u8]) -> String {
    use md5::{Digest, Md5};
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

static CLIENTS: OnceLock<HashMap<&'static str, &'static reqwest::blocking::Client>> = OnceLock::new();

/// 每平台一个长驻 blocking Client（'static：Box::leak 泄漏所有权）。
/// netease / qqlogin 不开 cookie 罐（登录凭据显式携带），qq / kugou / common 开
/// cookie 罐承接会话；qqlogin 专供 QQ 扫码登录阶段（显式 Cookie 头不被罐干扰）。
fn http(platform: &'static str) -> &'static reqwest::blocking::Client {
    let map = CLIENTS.get_or_init(|| {
        let mut m = HashMap::new();
        for p in ["netease", "qq", "kugou", "common", "qqlogin"] {
            let client: &'static reqwest::blocking::Client = Box::leak(Box::new(
                reqwest::blocking::Client::builder()
                    .cookie_store(matches!(p, "qq" | "kugou" | "common"))
                    .connect_timeout(std::time::Duration::from_secs(10))
                    .timeout(std::time::Duration::from_secs(20))
                    .build()
                    .unwrap_or_else(|_| reqwest::blocking::Client::new()),
            ));
            m.insert(p, client);
        }
        m
    });
    map.get(platform).copied().unwrap_or_else(|| {
        // 未知平台名（理论不可达）：退回无 cookie 的公共 client
        map["common"]
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
        // 关键：写注册表，否则歌单点歌时 music_play_url 查不到 key
        // 报「曲目信息已过期」（实测 bug：搜索能放、歌单静默失败）
        registry_put(&ordered);
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
                    registry_put(&tracks);
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
        registry_put(&tracks);
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
                eapi_post(
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
        // 游客通道（未登录或登录后由调用方 song_url_logged 优先）
        let guid: String = rand_digits(10);
        let media = if media_mid.is_empty() { song_mid.to_string() } else { media_mid.to_string() };
        // 免登录两通道：桌面（ct19/cv2201 + UrlGetVkey）优先，web（CgiGetVkey
        // + loginflag/platform）兜底；filename 高→低三档，midurlinfo 里第一
        // 个有 purl 的就是能拿到的最高档（实测桌面通道才给免费歌地址）
        let files: [(&str, &str); 3] = [("F000", "flac"), ("M800", "mp3"), ("M500", "mp3")];
        let mut last_err = String::new();
        for desktop in [true, false] {
            let mut param = json!({
                "guid": guid, "uin": "0", "ctx": 0,
                "filename": files.iter().map(|(p, e)| format!("{p}{media}.{e}")).collect::<Vec<_>>(),
                "songmid": [song_mid, song_mid, song_mid],
                "songtype": [0, 0, 0],
            });
            let (module, method, ct, cv) = if desktop {
                ("music.vkey.GetVkey", "UrlGetVkey", 19, 2201)
            } else {
                param["loginflag"] = json!(1);
                param["platform"] = json!("20");
                ("vkey.GetVkeyServer", "CgiGetVkey", 24, 0)
            };
            let payload = json!({
                "comm": { "ct": ct, "cv": cv },
                "req_0": { "module": module, "method": method, "param": param },
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
                last_err = format!("QQ 音乐播放请求被拒绝（code {}）", root["code"].as_i64().unwrap_or(-1));
                continue;
            }
            let data = &root["req_0"]["data"];
            let hosts: Vec<String> = data["sip"]
                .as_array()
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();
            for item in data["midurlinfo"].as_array().unwrap_or(&Vec::new()) {
                if item["result"].as_i64().unwrap_or(0) != 0 {
                    continue;
                }
                let purl = item["purl"].as_str().unwrap_or_default();
                if purl.is_empty() {
                    continue;
                }
                let mut candidates: Vec<String> = if purl.starts_with("https://") || purl.starts_with("http://") {
                    vec![purl.to_string()]
                } else {
                    let mut hs: Vec<&String> = hosts.iter().collect();
                    hs.sort_by_key(|h| if h.starts_with("https://") { 0 } else { 1 });
                    let mut list: Vec<String> = hs
                        .into_iter()
                        .map(|h| format!("{}{}", h.trim_end_matches('/'), purl))
                        .collect();
                    list.push(format!("https://isure.stream.qqmusic.qq.com/{purl}"));
                    list
                };
                candidates.retain(|c| c.ends_with(".mp3") || c.ends_with(".flac"));
                if let Some(candidate) = candidates.into_iter().next() {
                    let format = if candidate.ends_with(".flac") { "flac" } else { "mp3" };
                    return Ok(json!({ "url": candidate, "trial": false, "format": format }));
                }
            }
            last_err = "QQ 音乐没有返回可播放的音频".into();
        }
        Err(if last_err.is_empty() {
            "QQ 音乐没有返回可播放的音频（会员歌曲需要登录）".into()
        } else {
            last_err
        })
    }

    /* ---- 扫码登录（App 通道：CreateQRCode → MQTT5/WS 事件 → Login 换票） ---- */

    /// QQ g_tk：hash33（seed 5381，hash*33+code），取无符号 31 位
    fn hash33(value: &str) -> i64 {
        let mut hash: i64 = 5381;
        for ch in value.chars() {
            hash = hash.wrapping_mul(33).wrapping_add(ch as i64);
        }
        hash & 0x7fff_ffff
    }

    /// musicu.fcg 回包：req_0.code（根 code 非 0 时以根为准）
    fn fcg_code(root: &Value) -> i64 {
        let root_code = root["code"].as_i64().unwrap_or(0);
        if root_code != 0 {
            return root_code;
        }
        root["req_0"]["code"].as_i64().unwrap_or(-1)
    }

    /// GetLoginUserInfo 验证登录态；code==0 且昵称非空即有效
    fn verify_session(uin: &str, key: &str, login_type: i64) -> Result<(i64, Value), String> {
        let comm = json!({
            "ct": 19, "cv": 2201,
            "uin": uin, "authst": key,
            "g_tk": hash33(key), "tmeLoginType": login_type,
        });
        let payload = json!({
            "comm": comm,
            "req_0": { "module": "music.UserInfo.userInfoServer",
                       "method": "GetLoginUserInfo", "param": {} },
        });
        let resp = http("qqlogin")
            .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
            .header("Content-Type", "application/json")
            .header("Referer", REFERER)
            .header("User-Agent", UA)
            .header("Cookie", format!("qqmusic_uin={uin}; qqmusic_key={key};"))
            .body(payload.to_string())
            .send()
            .map_err(|e| e.to_string())?;
        let root: Value = resp.json().map_err(|e| e.to_string())?;
        let code = fcg_code(&root);
        Ok((code, root["req_0"]["data"].clone()))
    }

    fn profile_str(data: &Value, fields: &[&str]) -> String {
        for f in fields {
            if let Some(v) = data[*f].as_str() {
                if !v.is_empty() {
                    return v.to_string();
                }
            }
        }
        // GetLoginUserInfo 的昵称藏在 data.info（或 .base）下
        for child_key in ["info", "userInfo", "userinfo", "base"] {
            let child = &data[child_key];
            for f in fields {
                if let Some(v) = child[*f].as_str() {
                    if !v.is_empty() {
                        return v.to_string();
                    }
                }
            }
            if let Some(v) = child["base"][fields[0]].as_str() {
                if !v.is_empty() {
                    return v.to_string();
                }
            }
        }
        String::new()
    }

    pub fn qr_start(app: tauri::AppHandle) {
        std::thread::spawn(move || {
            let session = super::login_session_id();
            super::set_login("qq", "loading", "", "", "");
            if let Err(e) = qq_login_flow(&app, session) {
                super::set_login("qq", "error", "", "", &e);
            }
        });
    }

    fn qq_login_flow(app: &tauri::AppHandle, session: u64) -> Result<(), String> {
        // 1) CreateQRCode：拿 qrcodeID + base64 PNG（param 里必须带 ct/cv，
        //    否则手机端授权页 104610 拒绝）
        let root = login_fgc_qr_create()?;
        let data = root["req_0"]["data"].clone();
        let id = data["qrcodeID"].as_str().unwrap_or_default().to_string();
        let encoded = data["qrcode"].as_str().unwrap_or_default().to_string();
        if id.is_empty() || encoded.is_empty() {
            return Err("QQ 未返回二维码".into());
        }
        let png = encoded.split(',').next_back().unwrap_or_default();
        let png = base64::engine::general_purpose::STANDARD
            .decode(png)
            .map_err(|e| format!("二维码解码失败: {e}"))?;
        if png.is_empty() || png.len() > 1 << 20 {
            return Err("二维码数据异常".into());
        }

        // 2) MQTT5-over-WebSocket 事件通道（先订阅再亮码；mu.y.qq.com /ws/handshake）
        let mut channel = QqMqttChannel::connect(&id).map_err(|e| format!("QQ 登录通道连接失败: {e}"))?;
        if super::login_session_id() != session {
            return Ok(()); // 已被新一轮登录取代
        }
        let png_data = format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&png));
        super::set_login("qq", "waiting", "", &png_data, "");

        // 3) 事件循环（180s 超时；scanned / timeout / canceled / cookies）
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
        while super::login_session_id() == session && std::time::Instant::now() < deadline {
            let Some(event) = channel.next(deadline).map_err(|e| format!("QQ 登录通道错误: {e}"))? else {
                super::set_login("qq", "expired", "", "", "");
                return Ok(());
            };
            match event.0.as_str() {
                "scanned" => super::set_login("qq", "scanned", "", &png_data, ""),
                "timeout" => {
                    super::set_login("qq", "expired", "", "", "");
                    return Ok(());
                }
                "canceled" => {
                    super::set_login("qq", "error", "", "", "已在手机上取消登录，请刷新重试");
                    return Ok(());
                }
                "loginFailed" => return Err("QQ 音乐拒绝了本次登录，请检查账号后重试".into()),
                "cookies" => {
                    // 手机确认：cookies.qmusic_uin/qqmusic_key 推送过来
                    let payload: Value = serde_json::from_slice(&event.1)
                        .map_err(|e| format!("登录回包解析失败: {e}"))?;
                    let user = payload["cookies"]["qqmusic_uin"]["value"]
                        .as_str().unwrap_or_default().to_string();
                    let ticket = payload["cookies"]["qqmusic_key"]["value"]
                        .as_str().unwrap_or_default().to_string();
                    if user.is_empty() || ticket.is_empty() {
                        return Err("手机确认回包缺少登录凭据，请重试".into());
                    }
                    let account = qq_exchange(&user, &ticket, &id)?;
                    super::save_account("qq", &account, app);
                    super::set_login("qq", "success", "", "", "");
                    return Ok(());
                }
                _ => {}
            }
        }
        if super::login_session_id() == session {
            super::set_login("qq", "expired", "", "", "");
        }
        Ok(())
    }

    fn login_fgc_qr_create() -> Result<Value, String> {
        let payload = json!({
            "comm": { "ct": 23, "cv": 0 },
            "req_0": { "module": "music.login.LoginServer", "method": "CreateQRCode",
                        "param": { "tmeAppID": "qqmusic", "ct": 19, "cv": 2201 } },
        });
        let resp = http("qqlogin")
            .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
            .header("Content-Type", "application/json")
            .header("Referer", REFERER)
            .header("User-Agent", UA)
            .body(payload.to_string())
            .send()
            .map_err(|e| e.to_string())?;
        let root: Value = resp.json().map_err(|e| e.to_string())?;
        match fcg_code(&root) {
            0 => Ok(root),
            20279 => Err("QQ 音乐设备数已达上限，请在 App 里移除不用的设备后重试".into()),
            104604 => Err("登录尝试过于频繁，请稍后再试".into()),
            1000 | 104400 | 104401 => Err("二维码已过期，请刷新".into()),
            c => Err(format!("QQ 音乐拒绝了二维码请求（code {c}）")),
        }
    }

    /// Login 换票（换到的 key 自带 refresh_key 续期凭证）+ 验证；失败退回手机推来的原始 key
    fn qq_exchange(user: &str, ticket: &str, qr_id: &str) -> Result<super::Account, String> {
        let uin: i64 = user.parse().map_err(|_| "QQ 号解析失败".to_string())?;
        let param = json!({
            "musicid": uin, "qrCodeID": qr_id, "token": ticket,
            "loginType": 6, "needCookie": 1,
        });
        let root = login_fgc_login(param);
        if let Ok(r) = &root {
            if fcg_code(r) == 0 {
                let data = &r["req_0"]["data"];
                let id = data["musicid"].as_str()
                    .or_else(|| data["str_musicid"].as_str())
                    .unwrap_or_default()
                    .to_string();
                let key = data["musickey"].as_str().unwrap_or_default().to_string();
                if !id.is_empty() && !key.is_empty() {
                    let login_type = data["loginType"].as_i64().unwrap_or(if key.starts_with("W_X") { 1 } else { 2 });
                    // 会话传播可能短暂延迟：1000/104400/104401 重试一次
                    let mut verified = verify_session(&id, &key, login_type).unwrap_or((-1, Value::Null));
                    if [1000, 104400, 104401].contains(&verified.0) {
                        std::thread::sleep(std::time::Duration::from_millis(300));
                        verified = verify_session(&id, &key, login_type).unwrap_or((-1, Value::Null));
                    }
                    if verified.0 == 0 {
                        let nickname = profile_str(&verified.1, &["nick", "nickname", "nickName"]);
                        return Ok(super::Account {
                            user_id: id,
                            token: key,
                            nickname,
                            device: String::new(),
                            cookie: String::new(), // token 字段即 musickey；播放时显式拼 cookie
                        });
                    }
                }
            }
        }
        // 换票失败：手机推的原始 key 直接用（三天后需重新扫码）
        let login_type = if ticket.starts_with("W_X") { 1 } else { 2 };
        let (code, profile) = verify_session(user, ticket, login_type).map_err(|e| format!("登录验证失败: {e}"))?;
        if code != 0 {
            return Err("手机已确认，但桌面端登录验证未通过，请重试".into());
        }
        let nickname = profile_str(&profile, &["nick", "nickname", "nickName"]);
        Ok(super::Account {
            user_id: user.to_string(),
            token: ticket.to_string(),
            nickname,
            device: String::new(),
            cookie: String::new(),
        })
    }

    fn login_fgc_login(param: Value) -> Result<Value, String> {
        let payload = json!({
            "comm": { "ct": 19, "cv": 2201, "tmeLoginType": 6 },
            "req_0": { "module": "music.login.LoginServer", "method": "Login", "param": param },
        });
        let resp = http("qqlogin")
            .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
            .header("Content-Type", "application/json")
            .header("Referer", REFERER)
            .header("User-Agent", UA)
            .body(payload.to_string())
            .send()
            .map_err(|e| e.to_string())?;
        resp.json().map_err(|e| e.to_string())
    }

    pub fn account(app: &tauri::AppHandle) -> Result<Value, String> {
        match super::load_account("qq", app) {
            Some(a) if !a.token.is_empty() && !a.user_id.is_empty() => {
                let login_type = if a.token.starts_with("W_X") { 1 } else { 2 };
                let (code, profile) = verify_session(&a.user_id, &a.token, login_type).unwrap_or((-1, Value::Null));
                if code == 0 {
                    let nickname = profile_str(&profile, &["nick", "nickname", "nickName"]);
                    Ok(json!({ "loggedIn": true, "nickname": if nickname.is_empty() { a.nickname } else { nickname } }))
                } else {
                    // musickey 失效（三天未续）：视为未登录
                    Ok(json!({ "loggedIn": false, "note": "QQ 登录已过期，请重新扫码" }))
                }
            }
            _ => Ok(json!({ "loggedIn": false })),
        }
    }

    /// 登录后的 GetVkey：comm 带 uin/authst/g_tk，purl 覆盖面远大于游客通道
    pub fn song_url_logged(account: &super::Account, media_mid: &str, song_mid: &str) -> Result<Value, String> {
        let guid: String = rand_digits(10);
        let media = if media_mid.is_empty() { song_mid.to_string() } else { media_mid.to_string() };
        let files: [(&str, &str); 3] = [("F000", "flac"), ("M800", "mp3"), ("M500", "mp3")];
        let login_type = if account.token.starts_with("W_X") { 1 } else { 2 };
        let payload = json!({
            "comm": {
                "ct": 19, "cv": 2201,
                "uin": account.user_id, "authst": account.token,
                "g_tk": hash33(&account.token), "tmeLoginType": login_type,
            },
            "req_0": {
                "module": "music.vkey.GetVkey", "method": "UrlGetVkey",
                "param": {
                    "guid": guid, "uin": account.user_id, "ctx": 0,
                    "filename": files.iter().map(|(p, e)| format!("{p}{media}.{e}")).collect::<Vec<_>>(),
                    "songmid": [song_mid, song_mid, song_mid],
                    "songtype": [0, 0, 0],
                }
            },
        });
        let resp = http("qqlogin")
            .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
            .header("Content-Type", "application/json")
            .header("Referer", REFERER)
            .header("User-Agent", UA)
            .header("Cookie", format!("qqmusic_uin={}; qqmusic_key={};", account.user_id, account.token))
            .body(payload.to_string())
            .send()
            .map_err(|e| e.to_string())?;
        let root: Value = resp.json().map_err(|e| e.to_string())?;
        if fcg_code(&root) == 301 {
            return Err("QQ 登录已过期，请重新扫码".into());
        }
        if root["code"].as_i64() != Some(0) {
            return Err(format!("QQ 音乐播放请求被拒绝（code {}）", root["code"].as_i64().unwrap_or(-1)));
        }
        let data = &root["req_0"]["data"];
        let hosts: Vec<String> = data["sip"]
            .as_array()
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();
        for item in data["midurlinfo"].as_array().unwrap_or(&Vec::new()) {
            if item["result"].as_i64().unwrap_or(0) != 0 {
                continue;
            }
            let purl = item["purl"].as_str().unwrap_or_default();
            if purl.is_empty() {
                continue;
            }
            let mut candidates: Vec<String> = if purl.starts_with("https://") || purl.starts_with("http://") {
                vec![purl.to_string()]
            } else {
                let mut hs: Vec<&String> = hosts.iter().collect();
                hs.sort_by_key(|h| if h.starts_with("https://") { 0 } else { 1 });
                hs.into_iter().map(|h| format!("{}{}", h.trim_end_matches('/'), purl)).collect()
            };
            candidates.retain(|c| c.ends_with(".mp3") || c.ends_with(".flac"));
            if let Some(candidate) = candidates.into_iter().next() {
                let format = if candidate.ends_with(".flac") { "flac" } else { "mp3" };
                return Ok(json!({ "url": candidate, "trial": item["freeTrialInfo"].is_object(), "format": format }));
            }
        }
        Err("QQ 音乐没有返回可播放的音频".into())
    }

    /* ---- MQTT 5 over WebSocket（mu.y.qq.com，Sec-WebSocket-Protocol: mqtt） ---- */

    struct QqMqttChannel {
        stream: native_tls::TlsStream<std::net::TcpStream>,
        read_buf: Vec<u8>,
        fragment: Option<Vec<u8>>,
        topic: String,
        last_ping: std::time::Instant,
        ping_interval: std::time::Duration,
    }

    impl QqMqttChannel {
        /// TLS + WebSocket 升级 + MQTT CONNECT/SUBSCRIBE；reason 0x9c/0x9d 重定向同主机路径
        fn connect(qr_id: &str) -> Result<QqMqttChannel, String> {
            use native_tls::TlsConnector;
            use std::io::{Read, Write};

            let mut path = "/ws/handshake".to_string();
            for attempt in 0..4 {
                let connector = TlsConnector::new().map_err(|e| e.to_string())?;
                let tcp = std::net::TcpStream::connect("mu.y.qq.com:443")
                    .map_err(|e| format!("连接 mu.y.qq.com 失败: {e}"))?;
                tcp.set_read_timeout(Some(std::time::Duration::from_secs(8))).map_err(|e| e.to_string())?;
                let mut tls: native_tls::TlsStream<std::net::TcpStream> =
                    connector.connect("mu.y.qq.com", tcp).map_err(|e| format!("TLS 失败: {e}"))?;
                // WebSocket 升级
                let mut key_bytes = [0u8; 16];
                rand_bytes(&mut key_bytes);
                let ws_key = base64::engine::general_purpose::STANDARD.encode(key_bytes);
                let req = format!(
                    "GET {path} HTTP/1.1\r\nHost: mu.y.qq.com\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
                     Sec-WebSocket-Key: {ws_key}\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Protocol: mqtt\r\n\
                     Origin: https://y.qq.com\r\nReferer: https://y.qq.com/\r\nUser-Agent: Mozilla/5.0\r\n\r\n"
                );
                tls.write_all(req.as_bytes()).map_err(|e| e.to_string())?;
                // 读到 \r\n\r\n
                let mut head = Vec::new();
                let mut byte = [0u8; 1];
                loop {
                    tls.read(&mut byte).map_err(|e| e.to_string())?;
                    head.push(byte[0]);
                    if head.len() > 16384 {
                        return Err("握手响应过长".into());
                    }
                    if head.len() >= 4 && &head[head.len() - 4..] == b"\r\n\r\n" {
                        break;
                    }
                }
                let head_str = String::from_utf8_lossy(&head);
                let status = head_str.lines().next().unwrap_or_default();
                if !status.contains("101") {
                    return Err(format!("WebSocket 升级被拒: {}", status.trim()));
                }
                let mut accept_ok = false;
                let mut proto_ok = false;
                for line in head_str.lines().skip(1) {
                    let lower = line.to_ascii_lowercase();
                    if lower.starts_with("sec-websocket-accept:") {
                        // 校验 SHA1(key + GUID)
                        use sha1::{Digest, Sha1};
                        let mut hasher = Sha1::new();
                        hasher.update(ws_key.as_bytes());
                        hasher.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
                        let expect = base64::engine::general_purpose::STANDARD.encode(hasher.finalize());
                        let got = line.splitn(2, ':').nth(1).unwrap_or_default().trim();
                        accept_ok = got == expect;
                    } else if lower.starts_with("sec-websocket-protocol:") && lower.contains("mqtt") {
                        proto_ok = true;
                    }
                }
                if !accept_ok || !proto_ok {
                    return Err("WebSocket 握手校验失败".into());
                }

                // MQTT CONNECT（properties 里必须带 tmeAppID/hashTag/userID）
                let mut props = Vec::new();
                mqtt_push_prop_string(&mut props, "pass");
                mqtt_push_prop_user(&mut props, "tmeAppID", "qqmusic");
                mqtt_push_prop_user(&mut props, "business", "management");
                mqtt_push_prop_user(&mut props, "hashTag", qr_id);
                mqtt_push_prop_user(&mut props, "clientTag", "management.user");
                mqtt_push_prop_user(&mut props, "userID", qr_id);
                let client_id = format!("{}{}", now_millis(), rand_digits(4));
                let mut body = Vec::new();
                mqtt_push_string(&mut body, "MQTT");
                body.push(5); // protocol version
                body.push(2); // clean start
                mqtt_push_u16(&mut body, 45); // keepalive
                mqtt_push_varint(&mut body, props.len());
                body.extend_from_slice(&props);
                mqtt_push_string(&mut body, &client_id);
                let mut connect = vec![0x10];
                mqtt_push_varint(&mut connect, body.len());
                connect.extend_from_slice(&body);
                ws_send(&mut tls, 2, &connect).map_err(|e| e.to_string())?;

                let ack = Self::next_mqtt_packet_blocking(&mut tls)
                    .ok_or_else(|| "MQTT CONNECT 无响应".to_string())?;
                if ack[0] >> 4 != 2 {
                    return Err("MQTT 连接被拒".into());
                }
                let (reason, server_props) = mqtt_parse_connack(mqtt_packet_body(&ack)?)?;
                if reason == 0x9c || reason == 0x9d {
                    let reference = server_props.get("server").ok_or("缺少重定向路由")?;
                    if attempt == 3 {
                        return Err("QQ 登录通道重定向次数过多".into());
                    }
                    path = format!("/ws/handshake/{reference}");
                    continue; // 同一 TLS 主机的新路径重连
                }
                if reason != 0 {
                    return Err("MQTT 连接被服务器拒绝".into());
                }
                let keepalive_secs: u64 = server_props
                    .get("keepalive")
                    .and_then(|k| k.parse().ok())
                    .unwrap_or(20);
                let ping_interval = std::time::Duration::from_millis((keepalive_secs * 500).clamp(1000, 20000));

                // SUBSCRIBE：topic management.qrcode_login/{qrId}
                let topic = format!("management.qrcode_login/{qr_id}");
                let mut sub_props = Vec::new();
                mqtt_push_prop_user(&mut sub_props, "authorization", "tmelogin");
                mqtt_push_prop_user(&mut sub_props, "pubsub", "unicast");
                let mut sub_body = Vec::new();
                mqtt_push_u16(&mut sub_body, 1); // packet id
                mqtt_push_varint(&mut sub_body, sub_props.len());
                sub_body.extend_from_slice(&sub_props);
                mqtt_push_string(&mut sub_body, &topic);
                sub_body.push(0); // QoS 0
                let mut sub = vec![0x82];
                mqtt_push_varint(&mut sub, sub_body.len());
                sub.extend_from_slice(&sub_body);
                ws_send(&mut tls, 2, &sub).map_err(|e| e.to_string())?;

                let ack = Self::next_mqtt_packet_blocking(&mut tls)
                    .ok_or_else(|| "MQTT SUBSCRIBE 无响应".to_string())?;
                if ack[0] >> 4 != 9 {
                    return Err("MQTT 订阅被拒".into());
                }
                // SUBACK body: pid(2) + props(varint) + reason；reason ≥ 0x80 即订阅失败
                if let Some(&reason) = ack.iter().last() {
                    if reason >= 0x80 {
                        return Err("QQ 登录通道订阅被拒".into());
                    }
                }
                tls.set_read_timeout(Some(std::time::Duration::from_secs(1))).map_err(|e| e.to_string())?;
                return Ok(QqMqttChannel {
                    stream: tls,
                    read_buf: Vec::new(),
                    fragment: None,
                    topic,
                    last_ping: std::time::Instant::now(),
                    ping_interval,
                });
            }
            Err("QQ 登录通道连接失败".into())
        }

        /// 读一个 MQTT 包（deadline 前持续读），None = 超时
        fn next_packet_by_deadline(&mut self, deadline: std::time::Instant) -> Result<Option<Vec<u8>>, String> {
            use std::io::Read;
            loop {
                // 发 PINGREQ 保活
                if self.last_ping.elapsed() >= self.ping_interval {
                    let ping = vec![0xc0, 0x00];
                    ws_send(&mut self.stream, 2, &ping).map_err(|e| e.to_string())?;
                    self.last_ping = std::time::Instant::now();
                }
                // 缓冲里已经有完整 MQTT 包？
                if self.read_buf.len() >= 2 {
                    let mut length = 0usize;
                    let mut multiplier = 1usize;
                    let mut end = 1;
                    let mut body_known = false;
                    while end < self.read_buf.len() && end <= 4 {
                        let value = self.read_buf[end] & 0x7f;
                        length += (value as usize) * multiplier;
                        end += 1;
                        if self.read_buf[end - 1] & 0x80 == 0 {
                            body_known = true;
                            break;
                        }
                        multiplier *= 128;
                        if end > 4 {
                            return Err("MQTT 长度非法".into());
                        }
                    }
                    if body_known {
                        if self.read_buf.len() >= end + length {
                            let packet: Vec<u8> = self.read_buf[..end + length].to_vec();
                            self.read_buf.drain(..end + length);
                            return Ok(Some(packet));
                        }
                        // 包体不全：继续读
                    }
                }
                if std::time::Instant::now() >= deadline {
                    return Ok(None);
                }
                match ws_read_frame(&mut self.stream, &mut self.fragment, deadline) {
                    Ok(Some(data)) => {
                        if self.read_buf.len() + data.len() > 1 << 20 {
                            return Err("MQTT 缓冲过大".into());
                        }
                        self.read_buf.extend_from_slice(&data);
                    }
                    Ok(None) => continue, // ping/pong/片段
                    Err(e) => return Err(e),
                }
            }
        }

        /// 下一个登录事件：Some((type, payload)) / None = 超时
        fn next(&mut self, deadline: std::time::Instant) -> Result<Option<(String, Vec<u8>)>, String> {
            while std::time::Instant::now() < deadline {
                let packet = self.next_packet_by_deadline(deadline)?;
                let Some(packet) = packet else { return Ok(None) };
                let header = packet[0];
                match header >> 4 {
                    3 => {
                        // PUBLISH：topic → [qos1 packet id] → properties(type=user prop) → payload
                        let body = mqtt_packet_body(&packet)?;
                        let (topic, qos, after_topic) = mqtt_parse_publish(body, header)?;
                        if topic != self.topic {
                            continue;
                        }
                        let mut cursor = after_topic;
                        if qos == 1 {
                            if body.len() < cursor + 2 {
                                return Err("PUBLISH 包 id 越界".into());
                            }
                            let pid = ((body[cursor] as usize) << 8) | body[cursor + 1] as usize;
                            cursor += 2;
                            let ack = vec![0x40, 0x02, (pid >> 8) as u8, (pid & 0xff) as u8];
                            ws_send(&mut self.stream, 2, &ack).map_err(|e| e.to_string())?;
                        }
                        let (props, rest) = mqtt_parse_properties(&body[cursor..])?;
                        let event_type = props.get("type").cloned().unwrap_or_default();
                        return Ok(Some((event_type, rest.to_vec())));
                    }
                    13 => {} // PINGRESP
                    14 => return Err("QQ 登录通道断开".into()),
                    _ => return Err("MQTT 意外包类型".into()),
                }
            }
            Ok(None)
        }

        /// 握手阶段阻塞读第一个完整 MQTT 包（socket 已设 8s 超时）
        fn next_mqtt_packet_blocking(
            stream: &mut native_tls::TlsStream<std::net::TcpStream>,
        ) -> Option<Vec<u8>> {
            let mut pending: Vec<u8> = Vec::new();
            let mut fragment: Option<Vec<u8>> = None;
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
            while std::time::Instant::now() < deadline {
                if let Some(data) = ws_read_frame(stream, &mut fragment, deadline).ok().flatten() {
                    pending.extend_from_slice(&data);
                    if pending.len() >= 2 {
                        let mut length = 0usize;
                        let mut multiplier = 1usize;
                        let mut end = 1;
                        let mut done = false;
                        while end < pending.len() && end <= 4 {
                            let value = pending[end] & 0x7f;
                            length += (value as usize) * multiplier;
                            end += 1;
                            if pending[end - 1] & 0x80 == 0 {
                                done = true;
                                break;
                            }
                            multiplier *= 128;
                        }
                        if done && pending.len() >= end + length {
                            return Some(pending[..end + length].to_vec());
                        }
                    }
                }
            }
            None
        }
    }

    fn rand_bytes(out: &mut [u8]) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        for b in out.iter_mut() {
            *b = rng.gen();
        }
    }

    fn mqtt_push_string(out: &mut Vec<u8>, s: &str) {
        let bytes = s.as_bytes();
        mqtt_push_u16(out, bytes.len());
        out.extend_from_slice(bytes);
    }

    fn mqtt_push_u16(out: &mut Vec<u8>, v: usize) {
        out.push((v >> 8) as u8);
        out.push((v & 0xff) as u8);
    }

    fn mqtt_push_varint(out: &mut Vec<u8>, v: usize) {
        let mut remaining = v;
        loop {
            let mut digit = remaining % 128;
            remaining /= 128;
            if remaining > 0 {
                digit |= 0x80;
            }
            out.push(digit);
            if remaining == 0 {
                break;
            }
        }
    }

    /// 0x26 user property（name, value 两字符串）
    fn mqtt_push_prop_user(out: &mut Vec<u8>, name: &str, value: &str) {
        out.push(0x26);
        mqtt_push_string(out, name);
        mqtt_push_string(out, value);
    }

    fn mqtt_push_prop_string(out: &mut Vec<u8>, value: &str) {
        out.push(0x15); // Authentication Method
        mqtt_push_string(out, value);
    }

    /// MQTT 完整包 = [头字节][varint 长度][body]；取 body 切片
    fn mqtt_packet_body(packet: &[u8]) -> Result<&[u8], String> {
        if packet.len() < 2 {
            return Err("MQTT 包过短".into());
        }
        let mut length = 0usize;
        let mut multiplier = 1usize;
        let mut pos = 1;
        loop {
            if pos >= packet.len() || pos > 4 {
                return Err("MQTT 长度非法".into());
            }
            let digit = packet[pos];
            pos += 1;
            length += ((digit & 0x7f) as usize) * multiplier;
            if digit & 0x80 == 0 {
                break;
            }
            multiplier *= 128;
        }
        if packet.len() < pos + length {
            return Err("MQTT 包体不完整".into());
        }
        Ok(&packet[pos..pos + length])
    }

    /// 解析 CONNACK：返回 (reason, properties)；properties 含 server/keepalive 等
    fn mqtt_parse_connack(body: &[u8]) -> Result<(u8, HashMap<String, String>), String> {
        // body: [ack_flags, reason, properties...]
        if body.len() < 2 {
            return Err("CONNACK 过短".into());
        }
        let reason = body[1];
        let (props, _) = mqtt_parse_properties(&body[2..])?;
        Ok((reason, props))
    }

    /// 解析 PUBLISH 头：返回 (topic, qos, 包 id/properties 起始下标)
    fn mqtt_parse_publish(body: &[u8], header: u8) -> Result<(String, u8, usize), String> {
        if body.len() < 2 {
            return Err("PUBLISH 过短".into());
        }
        let qos = ((header >> 1) & 0x03) as u8;
        if qos > 1 {
            return Err("MQTT QoS2 不支持".into());
        }
        let topic_len = ((body[0] as usize) << 8) | body[1] as usize;
        if body.len() < 2 + topic_len {
            return Err("PUBLISH topic 越界".into());
        }
        let topic = String::from_utf8_lossy(&body[2..2 + topic_len]).to_string();
        Ok((topic, qos, 2 + topic_len))
    }

    /// MQTT properties 段（varint 长度前缀）；只解析 0x26/0x1c/0x13，其余按宽度跳过
    fn mqtt_parse_properties(data: &[u8]) -> Result<(HashMap<String, String>, &[u8]), String> {
        let mut pos = 0usize;
        // varint
        let mut length = 0usize;
        let mut multiplier = 1usize;
        loop {
            if pos >= data.len() {
                return Err("properties 长度越界".into());
            }
            let digit = data[pos];
            pos += 1;
            length += ((digit & 0x7f) as usize) * multiplier;
            if digit & 0x80 == 0 {
                break;
            }
            multiplier *= 128;
            if multiplier > 1 << 21 {
                return Err("properties 长度非法".into());
            }
        }
        if pos + length > data.len() {
            return Err("properties 越界".into());
        }
        let end = pos + length;
        let mut values = HashMap::new();
        while pos < end {
            let kind = data[pos];
            pos += 1;
            match kind {
                0x26 => {
                    let (name, next) = mqtt_read_string(data, pos, end)?;
                    let (value, next2) = mqtt_read_string(data, next, end)?;
                    values.insert(name, value);
                    pos = next2;
                }
                0x1c => {
                    let (v, next) = mqtt_read_string(data, pos, end)?;
                    values.insert("server".into(), v);
                    pos = next;
                }
                0x13 => {
                    if pos + 2 > end {
                        return Err("keepalive 越界".into());
                    }
                    values.insert("keepalive".into(), format!("{}", ((data[pos] as usize) << 8) | data[pos + 1] as usize));
                    pos += 2;
                }
                0x01 | 0x17 | 0x19 | 0x24 | 0x25 | 0x28 | 0x29 | 0x2a => { pos += 1; }
                0x21 | 0x22 | 0x23 => { pos += 2; }
                0x02 | 0x11 | 0x18 | 0x27 => { pos += 4; }
                0x0b => {
                    let (_v, next) = mqtt_read_varint(data, pos, end)?;
                    pos = next;
                }
                0x03 | 0x08 | 0x12 | 0x15 | 0x1a | 0x1f => {
                    let (_v, next) = mqtt_read_string(data, pos, end)?;
                    pos = next;
                }
                0x09 | 0x16 => {
                    if pos + 2 > end {
                        return Err("binary 越界".into());
                    }
                    let n = ((data[pos] as usize) << 8) | data[pos + 1] as usize;
                    pos += 2 + n;
                }
                _ => return Err(format!("未知 MQTT property 0x{kind:02x}")),
            }
            if pos > end {
                return Err("property 越界".into());
            }
        }
        Ok((values, &data[end..]))
    }

    fn mqtt_read_string(data: &[u8], mut pos: usize, end: usize) -> Result<(String, usize), String> {
        if pos + 2 > end {
            return Err("字符串长度越界".into());
        }
        let len = ((data[pos] as usize) << 8) | data[pos + 1] as usize;
        pos += 2;
        if pos + len > end {
            return Err("字符串越界".into());
        }
        Ok((String::from_utf8_lossy(&data[pos..pos + len]).to_string(), pos + len))
    }

    fn mqtt_read_varint(data: &[u8], mut pos: usize, end: usize) -> Result<(usize, usize), String> {
        let mut value = 0usize;
        let mut multiplier = 1usize;
        loop {
            if pos >= end {
                return Err("varint 越界".into());
            }
            let digit = data[pos];
            pos += 1;
            value += ((digit & 0x7f) as usize) * multiplier;
            if digit & 0x80 == 0 {
                break;
            }
            multiplier *= 128;
        }
        Ok((value, pos))
    }

    /// WebSocket 帧发送（客户端必须掩码）
    fn ws_send(
        stream: &mut native_tls::TlsStream<std::net::TcpStream>,
        opcode: u8,
        payload: &[u8],
    ) -> std::io::Result<()> {
        use rand::Rng;
        use std::io::Write;
        let mut rng = rand::thread_rng();
        let mut mask = [0u8; 4];
        for b in mask.iter_mut() {
            *b = rng.gen();
        }
        let mut frame = Vec::with_capacity(payload.len() + 14);
        frame.push(0x80 | opcode);
        if payload.len() < 126 {
            frame.push(0x80 | payload.len() as u8);
        } else if payload.len() <= 0xffff {
            frame.push(0x80 | 126);
            frame.push((payload.len() >> 8) as u8);
            frame.push((payload.len() & 0xff) as u8);
        } else {
            frame.push(0x80 | 127);
            for shift in (0..8).rev() {
                frame.push(((payload.len() >> (shift * 8)) & 0xff) as u8);
            }
        }
        frame.extend_from_slice(&mask);
        for (i, b) in payload.iter().enumerate() {
            frame.push(*b ^ mask[i % 4]);
        }
        stream.write_all(&frame)?;
        stream.flush()
    }

    /// 读一帧 WebSocket：Some(payload) = 二进制帧；None = ping/pong/片段未完成
    fn ws_read_frame(
        stream: &mut native_tls::TlsStream<std::net::TcpStream>,
        fragment: &mut Option<Vec<u8>>,
        deadline: std::time::Instant,
    ) -> Result<Option<Vec<u8>>, String> {
        use std::io::Read;
        fn read_exact_timeout(
            stream: &mut native_tls::TlsStream<std::net::TcpStream>,
            buf: &mut [u8],
            deadline: std::time::Instant,
        ) -> Result<(), String> {
            let mut filled = 0;
            while filled < buf.len() {
                if std::time::Instant::now() >= deadline {
                    return Err("读超时".into());
                }
                match stream.read(&mut buf[filled..]) {
                    Ok(0) => return Err("连接关闭".into()),
                    Ok(n) => filled += n,
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        continue;
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }
            Ok(())
        }
        let mut hdr = [0u8; 2];
        read_exact_timeout(stream, &mut hdr, deadline)?;
        let first = hdr[0];
        let second = hdr[1];
        if first & 0x70 != 0 || second & 0x80 != 0 {
            return Err("服务端 WebSocket 帧非法".into());
        }
        let mut length = (second & 0x7f) as u64;
        if length == 126 {
            let mut ext = [0u8; 2];
            read_exact_timeout(stream, &mut ext, deadline)?;
            length = ((ext[0] as u64) << 8) | ext[1] as u64;
        } else if length == 127 {
            let mut ext = [0u8; 8];
            read_exact_timeout(stream, &mut ext, deadline)?;
            length = 0;
            for b in ext {
                length = (length << 8) | b as u64;
            }
        }
        if length > 1 << 20 {
            return Err("WebSocket 帧过大".into());
        }
        let opcode = first & 0x0f;
        let fin = first & 0x80 != 0;
        if opcode >= 8 && (!fin || length > 125) {
            return Err("控制帧非法".into());
        }
        let mut payload = vec![0u8; length as usize];
        read_exact_timeout(stream, &mut payload, deadline)?;
        match opcode {
            8 => Err("WebSocket 已被服务端关闭".into()),
            9 => {
                // PING → 回 PONG
                ws_send(stream, 10, &payload).map_err(|e| e.to_string())?;
                Ok(None)
            }
            10 => Ok(None),
            2 => {
                if fragment.is_some() {
                    return Err("意外的二进制片段".into());
                }
                if fin {
                    Ok(Some(payload))
                } else {
                    *fragment = Some(payload);
                    Ok(None)
                }
            }
            0 => {
                let partial = fragment.take().ok_or_else(|| "意外的续帧".to_string())?;
                let mut combined = partial;
                if combined.len() + payload.len() > 1 << 20 {
                    return Err("片段过大".into());
                }
                combined.extend_from_slice(&payload);
                if fin {
                    Ok(Some(combined))
                } else {
                    *fragment = Some(combined);
                    Ok(None)
                }
            }
            _ => Err("意外的 WebSocket 数据帧".into()),
        }
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
                            let fallback = json!({});
                            let info = poll["data"]["userinfo"]
                                .as_object()
                                .or_else(|| poll["data"]["user_info"].as_object())
                                .unwrap_or_else(|| {
                                    poll["data"].as_object().unwrap_or(fallback.as_object().unwrap())
                                });
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
        use md5::{Digest, Md5};
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
                // v3 搜索返回 duration（秒）；旧字段 timelen 直接当毫秒兜底
                let sec = s["duration"].as_i64().unwrap_or(0);
                let ms = if sec > 0 { sec * 1000 } else { s["timelen"].as_i64().unwrap_or(0) };
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
            for key in ["play_url", "url", "backup_url"] {
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
        // 歌词搜索按秒容差匹配（实测 duration=269 命中晴天；传毫秒会落空）
        let search_url = format!(
            "https://lyrics.kugou.com/search?ver=1&man=yes&client=pc&hash={}&duration={}",
            urlencode(hash),
            duration_ms / 1000
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
        if m < 0 || s < 0.0 {
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

fn registry_put(items: &[Value]) {
    let mut map = tracks().lock().unwrap_or_else(|e| e.into_inner());
    for t in items {
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
pub fn music_login_start(platform: String, app: tauri::AppHandle) -> Result<(), String> {
    if !platform_supported(&platform) {
        return Err(format!("{platform} 暂不支持"));
    }
    super_login_session_bump();
    match platform.as_str() {
        "netease" => music_platform::netease::qr_start(app),
        "qq" => music_platform::qq::qr_start(app),
        "kugou" => music_platform::kugou::qr_start(app),
        _ => return Err("不支持的平台".into()),
    }
    Ok(())
}

fn super_login_session_bump() {
    LOGIN_SESSION.fetch_add(1, Ordering::SeqCst);
}

#[tauri::command]
pub fn music_login_state() -> Value {
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
pub fn music_status(platform: String, app: tauri::AppHandle) -> Result<Value, String> {
    match platform.as_str() {
        "netease" => music_platform::netease::account(&app),
        "qq" => music_platform::qq::account(&app),
        "kugou" => music_platform::kugou::account(&app),
        _ => Err("不支持的平台".into()),
    }
}

#[tauri::command]
pub fn music_logout(platform: String, app: tauri::AppHandle) -> Result<(), String> {
    clear_account(&platform, &app);
    Ok(())
}

#[tauri::command]
pub fn music_playlists(platform: String, app: tauri::AppHandle) -> Result<Value, String> {
    match platform.as_str() {
        "netease" => music_platform::netease::playlists(&app),
        "kugou" => Err("酷狗歌单浏览暂未开放".into()),
        _ => Err("不支持的平台".into()),
    }
}

#[tauri::command]
pub fn music_playlist_tracks(platform: String, id: String, app: tauri::AppHandle) -> Result<Value, String> {
    match platform.as_str() {
        "netease" => music_platform::netease::playlist_tracks(&app, &id),
        _ => Err("不支持的平台".into()),
    }
}

#[tauri::command]
pub fn music_search(platform: String, query: String) -> Result<Value, String> {
    match platform.as_str() {
        "netease" => music_platform::netease::search(&query),
        "qq" => music_platform::qq::search(&query),
        "kugou" => music_platform::kugou::search(&query),
        _ => Err("不支持的平台".into()),
    }
}

#[tauri::command]
pub fn music_play_url(platform: String, key: String, app: tauri::AppHandle) -> Result<Value, String> {
    let track = registry_get(&key).ok_or("曲目信息已过期，请重新搜索")?;
    match platform.as_str() {
        "netease" => {
            let id = track["id"].as_str().unwrap_or_default().to_string();
            music_platform::netease::song_url(&app, &id)
        }
        "qq" => {
            let media = track["mediaMid"].as_str().unwrap_or_default().to_string();
            let mid = track["id"].as_str().unwrap_or_default().to_string();
            // 登录态优先（会员歌可放），失败退游客通道
            if let Some(account) = load_account("qq", &app) {
                if !account.token.is_empty() {
                    if let Ok(url) = music_platform::qq::song_url_logged(&account, &media, &mid) {
                        if !url["url"].as_str().unwrap_or_default().is_empty() {
                            return Ok(url);
                        }
                    }
                }
            }
            music_platform::qq::song_url(&media, &mid)
        }
        "kugou" => music_platform::kugou::song_url(&app, &track),
        _ => Err("不支持的平台".into()),
    }
}

#[tauri::command]
pub fn music_lyrics(platform: String, key: String) -> Result<Value, String> {
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
