use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn w(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

pub fn wp(p: &Path) -> Vec<u16> {
    w(&p.display().to_string())
}

pub fn fill(dst: &mut [u16], s: &str) {
    let mut i = 0;
    for c in s.encode_utf16() {
        if i + 1 >= dst.len() {
            break;
        }
        dst[i] = c;
        i += 1;
    }
    dst[i] = 0;
}

pub fn exe() -> PathBuf {
    std::env::current_exe().unwrap_or_default()
}

pub fn base() -> PathBuf {
    exe().parent().map(Path::to_path_buf).unwrap_or_default()
}

pub fn work() -> PathBuf {
    base().join("core")
}

pub fn pct(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            o.push(b as char);
        } else {
            o.push_str(&format!("%{b:02X}"));
        }
    }
    o
}

pub fn jstr(s: &str) -> String {
    let mut o = String::from('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

pub fn secret() -> String {
    let t = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
    let mut o = String::new();
    for i in 0..2u64 {
        let mut h = RandomState::new().build_hasher();
        h.write_u64(t ^ i);
        o.push_str(&format!("{:016x}", h.finish()));
    }
    o
}

pub fn free_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .unwrap_or(9097)
}

pub fn port_open(p: u16) -> bool {
    TcpListener::bind(("127.0.0.1", p)).is_ok() && TcpListener::bind(("0.0.0.0", p)).is_ok()
}

pub fn near_port(p: u16) -> u16 {
    (p..p.saturating_add(100)).find(|&p| port_open(p)).unwrap_or_else(free_port)
}
