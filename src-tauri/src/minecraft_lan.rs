//! Minecraft Java Edition LAN discovery.
//! Vanilla broadcasts [MOTD]…[/MOTD][AD]port[/AD] to 224.0.2.60:4445.
//! The optional server-list ping retrieves the same favicon used by the
//! vanilla multiplayer screen without requiring any Minecraft credentials.

use serde::Serialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, UdpSocket};
use std::sync::{LazyLock, RwLock};
use std::time::{Duration, Instant};

static GAMES: LazyLock<RwLock<HashMap<String, LanGame>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanGame {
    ip: String,
    port: u16,
    motd: String,
    cover: Option<String>,
    #[serde(skip)]
    last_seen: Instant,
}

pub fn init() {
    std::thread::spawn(|| {
        if let Err(error) = listen() {
            astral_core::api::simple::cmd_log(&format!("minecraft-lan: listener failed: {error}"));
        }
    });
}

fn listen() -> Result<(), String> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 4445)).map_err(|e| e.to_string())?;
    socket
        .join_multicast_v4(&Ipv4Addr::new(224, 0, 2, 60), &Ipv4Addr::UNSPECIFIED)
        .map_err(|e| e.to_string())?;
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    let mut buffer = [0_u8; 4096];
    loop {
        let (length, sender) = match socket.recv_from(&mut buffer) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => continue,
            Err(error) if error.kind() == std::io::ErrorKind::TimedOut => continue,
            Err(error) => return Err(error.to_string()),
        };
        let Some((motd, port)) = parse_advertisement(&buffer[..length]) else {
            continue;
        };
        let ip = sender.ip().to_string();
        let key = format!("{ip}:{port}");
        let previous = GAMES.read().ok().and_then(|games| games.get(&key).cloned());
        let cover = match previous {
            Some(game) => game.cover,
            None => query_cover(sender.ip(), port).unwrap_or(None),
        };
        let game = LanGame {
            ip,
            port,
            motd,
            cover,
            last_seen: Instant::now(),
        };
        if let Ok(mut games) = GAMES.write() {
            games.insert(key, game);
            games.retain(|_, value| value.last_seen.elapsed() < Duration::from_secs(20));
        }
    }
}

fn parse_advertisement(data: &[u8]) -> Option<(String, u16)> {
    let text = String::from_utf8_lossy(data);
    let motd = text.split("[MOTD]").nth(1)?.split("[/MOTD]").next()?.trim();
    let port = text
        .split("[AD]")
        .nth(1)?
        .split("[/AD]")
        .next()?
        .trim()
        .parse::<u16>()
        .ok()?;
    if motd.is_empty() || port == 0 {
        return None;
    }
    Some((motd.to_string(), port))
}

fn query_cover(ip: IpAddr, port: u16) -> Option<Option<String>> {
    let address = SocketAddr::new(ip, port);
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_millis(900)).ok()?;
    stream.set_read_timeout(Some(Duration::from_millis(900))).ok()?;
    stream.set_write_timeout(Some(Duration::from_millis(900))).ok()?;

    let host = ip.to_string();
    let mut handshake = Vec::new();
    put_varint(&mut handshake, 0);
    put_varint(&mut handshake, 765);
    put_varint(&mut handshake, host.len() as i32);
    handshake.extend_from_slice(host.as_bytes());
    handshake.extend_from_slice(&port.to_be_bytes());
    put_varint(&mut handshake, 1);

    let mut packet = Vec::new();
    put_varint(&mut packet, handshake.len() as i32);
    packet.extend_from_slice(&handshake);
    packet.extend_from_slice(&[1, 0]);
    stream.write_all(&packet).ok()?;
    stream.flush().ok()?;

    let packet_length = get_varint(&mut stream).ok()? as usize;
    if packet_length > 4 * 1024 * 1024 {
        return None;
    }
    let packet_id = get_varint(&mut stream).ok()?;
    if packet_id != 0 {
        return None;
    }
    let json_length = get_varint(&mut stream).ok()? as usize;
    if json_length > packet_length || json_length > 4 * 1024 * 1024 {
        return None;
    }
    let mut json = vec![0_u8; json_length];
    stream.read_exact(&mut json).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&json).ok()?;
    let cover = value.get("favicon")?.as_str()?.to_string();
    if cover.len() > 300_000 || !cover.starts_with("data:image/png;base64,") {
        return Some(None);
    }
    Some(Some(cover))
}

fn put_varint(output: &mut Vec<u8>, mut value: i32) {
    loop {
        if (value & !0x7f) == 0 {
            output.push(value as u8);
            return;
        }
        output.push(((value & 0x7f) as u8) | 0x80);
        value = ((value as u32) >> 7) as i32;
    }
}

fn get_varint(stream: &mut TcpStream) -> Result<i32, String> {
    let mut value = 0_i32;
    for index in 0..5 {
        let mut byte = [0_u8; 1];
        stream.read_exact(&mut byte).map_err(|e| e.to_string())?;
        value |= ((byte[0] & 0x7f) as i32) << (index * 7);
        if byte[0] & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err("invalid VarInt".to_string())
}

#[tauri::command]
pub fn minecraft_lan_games() -> Vec<LanGame> {
    let mut games = match GAMES.write() {
        Ok(games) => games,
        Err(_) => return Vec::new(),
    };
    games.retain(|_, game| game.last_seen.elapsed() < Duration::from_secs(20));
    let mut result = games.values().cloned().collect::<Vec<_>>();
    result.sort_by(|a, b| a.ip.cmp(&b.ip).then(a.port.cmp(&b.port)));
    result
}

#[cfg(test)]
mod tests {
    use super::parse_advertisement;

    #[test]
    fn parses_vanilla_lan_advertisement() {
        let data = b"[MOTD]A Minecraft World[/MOTD][AD]25565[/AD]";
        assert_eq!(parse_advertisement(data), Some(("A Minecraft World".into(), 25565)));
    }
}
