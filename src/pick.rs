use crate::ffi::*;
use crate::util::w;
use std::collections::BTreeMap;
use std::fs;
use std::mem::{size_of, zeroed};
use std::path::{Path, PathBuf};
use std::ptr::null_mut;

const JUNK: [&str; 14] = [
    "unins", "redist", "crash", "setup", "dxweb", "dotnet", "prereq", "report", "helper", "vc_", "easyanticheat", "battleye", "cefprocess", "update",
];

pub fn steam() -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for lib in libs() {
        let apps = lib.join("steamapps");
        let Ok(rd) = fs::read_dir(&apps) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let f = p.file_name().and_then(|f| f.to_str()).unwrap_or("");
            if !f.starts_with("appmanifest_") || !f.ends_with(".acf") {
                continue;
            }
            let Ok(t) = fs::read_to_string(&p) else { continue };
            let (mut name, mut dir) = (String::new(), String::new());
            for l in t.lines() {
                match kv(l) {
                    Some((k, v)) if k == "name" => name = v,
                    Some((k, v)) if k == "installdir" => dir = v,
                    _ => {}
                }
            }
            if name.is_empty() || dir.is_empty() || name.starts_with("Steamworks") {
                continue;
            }
            if let Some((x, path)) = main_exe(&apps.join("common").join(&dir), &[&name, &dir]) {
                out.push((name, x, path));
            }
        }
    }
    out.sort_by_key(|a| a.0.to_lowercase());
    out
}

fn libs() -> Vec<PathBuf> {
    let root = PathBuf::from(steam_path());
    let mut v = vec![root.clone()];
    if let Ok(t) = fs::read_to_string(root.join("steamapps").join("libraryfolders.vdf")) {
        for (k, p) in t.lines().filter_map(kv) {
            let p = PathBuf::from(p);
            if k == "path" && !v.iter().any(|x| same(x, &p)) {
                v.push(p);
            }
        }
    }
    v
}

fn same(a: &Path, b: &Path) -> bool {
    let n = |p: &Path| p.display().to_string().replace('/', "\\").trim_end_matches('\\').to_lowercase();
    n(a) == n(b)
}

fn steam_path() -> String {
    let mut k = null_mut();
    let mut buf = vec![0u16; 520];
    let mut len = (buf.len() * 2) as u32;
    let ok = unsafe {
        RegOpenKeyExW(hkcu(), w("Software\\Valve\\Steam").as_ptr(), 0, KEY_QUERY_VALUE, &mut k) == 0
            && RegQueryValueExW(k, w("SteamPath").as_ptr(), null_mut(), null_mut(), buf.as_mut_ptr() as *mut u8, &mut len) == 0
    };
    if !k.is_null() {
        unsafe { RegCloseKey(k) };
    }
    if !ok {
        return "C:\\Program Files (x86)\\Steam".into();
    }
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..n])
}

fn kv(l: &str) -> Option<(String, String)> {
    let p: Vec<&str> = l.trim().split('"').collect();
    if p.len() < 5 {
        return None;
    }
    Some((p[1].to_lowercase(), p[3].replace("\\\\", "\\")))
}

fn main_exe(dir: &Path, hints: &[&str]) -> Option<(String, String)> {
    let mut all = Vec::new();
    walk(dir, 0, &mut all);
    let hints: Vec<String> = hints.iter().map(|h| plain(h)).collect();
    all.into_iter()
        .max_by_key(|(depth, size, name, _)| {
            let stem = plain(&name[..name.len() - 4]);
            let score = hints.iter().map(|h| stem.chars().zip(h.chars()).take_while(|(a, b)| a == b).count()).max().unwrap_or(0);
            (score, u32::MAX - depth, *size)
        })
        .map(|b| (b.2, b.3))
}

fn plain(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

fn walk(dir: &Path, depth: u32, all: &mut Vec<(u32, u64, String, String)>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let Ok(t) = e.file_type() else { continue };
        let name = e.file_name().to_string_lossy().to_string();
        let low = name.to_lowercase();
        if t.is_dir() {
            if depth < 4 && !low.contains("redist") && !low.starts_with('_') {
                walk(&e.path(), depth + 1, all);
            }
            continue;
        }
        if low.ends_with(".exe") && !JUNK.iter().any(|j| low.contains(j)) {
            all.push((depth, e.metadata().map(|m| m.len()).unwrap_or(0), name, e.path().display().to_string()));
        }
    }
}

pub fn running() -> Vec<(String, String, String)> {
    let mut map = BTreeMap::new();
    let mut me = 0;
    unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut me) };
    let snap = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snap == invalid() {
        return Vec::new();
    }
    let mut e: PROCESSENTRY32W = unsafe { zeroed() };
    e.size = size_of::<PROCESSENTRY32W>() as u32;
    let mut more = unsafe { Process32FirstW(snap, &mut e) } != 0;
    while more {
        let mut s = u32::MAX;
        unsafe { ProcessIdToSessionId(e.pid, &mut s) };
        let n = e.exe.iter().position(|&c| c == 0).unwrap_or(e.exe.len());
        let name = String::from_utf16_lossy(&e.exe[..n]);
        if s == me && name.to_lowercase().ends_with(".exe") {
            let slot = map.entry(name.to_lowercase()).or_insert((name, String::new()));
            if slot.1.is_empty() {
                slot.1 = path(e.pid);
            }
        }
        more = unsafe { Process32NextW(snap, &mut e) } != 0;
    }
    unsafe { CloseHandle(snap) };
    map.into_values().map(|(n, p)| (n.clone(), n, p)).collect()
}

fn path(pid: u32) -> String {
    let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if h.is_null() {
        return String::new();
    }
    let mut buf = [0u16; 1024];
    let mut len = buf.len() as u32;
    let ok = unsafe { QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut len) } != 0;
    unsafe { CloseHandle(h) };
    if ok {
        String::from_utf16_lossy(&buf[..len as usize])
    } else {
        String::new()
    }
}
