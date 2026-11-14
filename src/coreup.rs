use crate::engine::Core;
use crate::ffi::{BCryptHash, BCRYPT_SHA256, CREATE_NO_WINDOW};
use crate::util::{base, work};
use crate::{json, net};
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const API: &str = "https://api.github.com/repos/MetaCubeX/mihomo/releases/latest";
const UA: &str = "ZonsVPN";
const DAY: u64 = 86_400;

fn next() -> PathBuf {
    work().join("next.exe")
}

fn prev() -> PathBuf {
    work().join("prev.exe")
}

fn state() -> PathBuf {
    work().join("update.txt")
}

fn target() -> PathBuf {
    Core::exe().unwrap_or_else(|| base().join("mihomo.exe"))
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn load() -> (u64, String) {
    let t = fs::read_to_string(state()).unwrap_or_default();
    let mut l = t.lines();
    (l.next().and_then(|s| s.parse().ok()).unwrap_or(0), l.next().unwrap_or("").to_string())
}

fn save(checked: u64, bad: &str) {
    let _ = fs::create_dir_all(work());
    let _ = fs::write(state(), format!("{checked}\n{bad}\n"));
}

fn native(p: &Path) -> bool {
    let mut b = [0u8; 1024];
    let n = fs::File::open(p).and_then(|mut f| std::io::Read::read(&mut f, &mut b)).unwrap_or(0);
    if n < 64 || &b[..2] != b"MZ" {
        return false;
    }
    let o = u32::from_le_bytes([b[60], b[61], b[62], b[63]]) as usize;
    if o + 6 > n || &b[o..o + 4] != b"PE\0\0" {
        return false;
    }
    let m = u16::from_le_bytes([b[o + 4], b[o + 5]]);
    m == if cfg!(target_arch = "aarch64") { 0xAA64 } else { 0x8664 }
}

pub fn version(p: &Path) -> Option<String> {
    if !native(p) {
        return None;
    }
    let o = Command::new(p).arg("-v").creation_flags(CREATE_NO_WINDOW).output().ok()?;
    let t = String::from_utf8_lossy(&o.stdout).into_owned();
    t.split_whitespace().find(|s| s.len() > 1 && s.starts_with('v') && s.as_bytes()[1].is_ascii_digit()).map(String::from)
}

pub fn current() -> String {
    Core::exe().and_then(|p| version(&p)).unwrap_or_default()
}

fn asset(tag: &str) -> String {
    let arch = if cfg!(target_arch = "aarch64") { "arm64" } else { "amd64-compatible" };
    format!("mihomo-windows-{arch}-{tag}.zip")
}

fn fetch(url: &str, proxy: Option<u16>) -> Result<Vec<u8>, String> {
    net::get(url, UA, None).or_else(|e| match proxy {
        Some(p) => net::get(url, UA, Some(p)),
        None => Err(e),
    })
}

fn sha256(data: &[u8]) -> Result<String, String> {
    let mut out = [0u8; 32];
    let r = unsafe { BCryptHash(BCRYPT_SHA256, std::ptr::null(), 0, data.as_ptr(), data.len() as u32, out.as_mut_ptr(), 32) };
    if r != 0 {
        return Err(format!("SHA-256: код {r:#x}"));
    }
    Ok(out.iter().map(|b| format!("{b:02x}")).collect())
}

fn unzip(zip: &[u8], dir: &Path) -> Result<PathBuf, String> {
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let z = dir.join("core.zip");
    fs::write(&z, zip).map_err(|e| e.to_string())?;
    let root = std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    let ok = Command::new(root.join("System32").join("tar.exe"))
        .arg("-xf")
        .arg(&z)
        .arg("-C")
        .arg(dir)
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|e| format!("tar.exe: {e}"))?
        .success();
    if !ok {
        return Err("не удалось распаковать архив ядра".into());
    }
    fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("exe")))
        .ok_or_else(|| "в архиве нет exe".into())
}

pub fn check(proxy: Option<u16>, force: bool) -> Result<Option<String>, String> {
    let (checked, bad) = load();
    let t = now();
    if !force && t < checked + DAY {
        return Ok(None);
    }
    let body = fetch(API, proxy)?;
    save(t, &bad);
    let v = json::parse(&String::from_utf8_lossy(&body)).ok_or("GitHub вернул непонятный ответ")?;
    let tag = v.get("tag_name").str().to_string();
    if tag.is_empty() {
        return Err("GitHub не назвал версию ядра".into());
    }
    if Core::exe().and_then(|p| version(&p)).as_deref() == Some(tag.as_str()) || (tag == bad && !force) {
        return Ok(None);
    }
    if version(&next()).as_deref() == Some(tag.as_str()) {
        return Ok(Some(tag));
    }
    let name = asset(&tag);
    let a = v.get("assets").arr().iter().find(|a| a.get("name").str() == name).ok_or(format!("в релизе нет {name}"))?;
    let want = a.get("digest").str().strip_prefix("sha256:").ok_or("у релиза нет SHA-256")?.to_ascii_lowercase();
    let zip = fetch(a.get("browser_download_url").str(), proxy)?;
    if sha256(&zip)? != want {
        return Err("SHA-256 скачанного ядра не совпал".into());
    }
    let dir = work().join("download");
    let r = unzip(&zip, &dir).and_then(|exe| {
        if version(&exe).as_deref() != Some(tag.as_str()) {
            return Err("скачанное ядро не запускается".into());
        }
        let _ = fs::remove_file(next());
        fs::rename(&exe, next()).map_err(|e| e.to_string())
    });
    let _ = fs::remove_dir_all(&dir);
    r.map(|_| Some(tag))
}

fn mv(a: &Path, b: &Path) -> bool {
    for _ in 0..20 {
        if fs::rename(a, b).is_ok() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    false
}

pub fn staged() -> bool {
    next().is_file()
}

pub fn apply() -> bool {
    let n = next();
    if !n.is_file() {
        return false;
    }
    if !native(&n) {
        let _ = fs::remove_file(&n);
        return false;
    }
    let t = target();
    let _ = fs::remove_file(prev());
    if t.is_file() && !mv(&t, &prev()) {
        return false;
    }
    if !mv(&n, &t) {
        mv(&prev(), &t);
        return false;
    }
    true
}

pub fn commit() {
    let _ = fs::remove_file(prev());
}

pub fn rollback() -> bool {
    let p = prev();
    if !p.is_file() {
        return false;
    }
    let t = target();
    let bad = version(&t).unwrap_or_default();
    if !mv(&p, &t) {
        return false;
    }
    save(load().0, &bad);
    true
}
