use crate::net;
use crate::util::{base, work};
use std::fs;
use std::path::{Path, PathBuf};

const UA: &str = "clash.meta";
const HEAD: &str = "# name url [user-agent]\n";

#[derive(Clone)]
pub struct Sub {
    pub name: String,
    pub url: String,
    pub ua: String,
}

pub fn dir() -> PathBuf {
    work().join("profiles")
}

pub fn run_path() -> PathBuf {
    work().join("run.yaml")
}

fn subs_file() -> PathBuf {
    base().join("subs.txt")
}

pub fn subs() -> Vec<Sub> {
    let t = fs::read_to_string(subs_file()).unwrap_or_default();
    t.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?.to_string();
            let url = it.next()?.to_string();
            let ua = it.collect::<Vec<_>>().join(" ");
            Some(Sub { name, url, ua: if ua.is_empty() { UA.into() } else { ua } })
        })
        .collect()
}

fn save_subs(v: &[Sub]) -> Result<(), String> {
    let mut o = String::from(HEAD);
    for s in v {
        if s.ua == UA {
            o.push_str(&format!("{} {}\n", s.name, s.url));
        } else {
            o.push_str(&format!("{} {} {}\n", s.name, s.url, s.ua));
        }
    }
    fs::write(subs_file(), o).map_err(|e| e.to_string())
}

pub fn list() -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir())
        .map(|d| {
            d.flatten()
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|n| {
                    let l = n.to_ascii_lowercase();
                    l.ends_with(".yaml") || l.ends_with(".yml")
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

pub fn path(file: &str) -> PathBuf {
    dir().join(file)
}

pub fn label(file: &str) -> String {
    Path::new(file).file_stem().and_then(|s| s.to_str()).unwrap_or(file).to_string()
}

fn safe(n: &str) -> String {
    n.chars().map(|c| if "<>:\"/\\|?*".contains(c) || c.is_control() || c.is_whitespace() { '_' } else { c }).collect()
}

pub fn file_of(name: &str) -> String {
    format!("{}.yaml", safe(name))
}

pub fn add(name: &str, url: &str) -> Result<String, String> {
    let url = url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("Р РЋРЎРѓРЎвЂ№Р В»Р С”Р В° Р Т‘Р С•Р В»Р В¶Р Р…Р В° Р Р…Р В°РЎвЂЎР С‘Р Р…Р В°РЎвЂљРЎРЉРЎРѓРЎРЏ РЎРѓ http:// Р С‘Р В»Р С‘ https://".into());
    }
    let mut name = safe(name.trim());
    if name.is_empty() {
        name = url.split("://").nth(1).and_then(|r| r.split(['/', '?', ':']).next()).map(safe).unwrap_or_else(|| "sub".into());
    }
    let mut v = subs();
    v.retain(|s| s.name != name);
    v.push(Sub { name: name.clone(), url: url.to_string(), ua: UA.into() });
    save_subs(&v)?;
    Ok(name)
}

pub fn remove(file: &str) -> Result<(), String> {
    let mut v = subs();
    v.retain(|s| file_of(&s.name) != file);
    save_subs(&v)?;
    let p = path(file);
    if p.exists() {
        fs::remove_file(p).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn is_config(t: &str) -> bool {
    t.lines().any(|l| l.starts_with("proxies:") || l.starts_with("proxy-providers:"))
}

fn is_links(t: &str) -> bool {
    let t = t.trim();
    !t.is_empty() && (t.contains("://") || t.bytes().all(|c| c.is_ascii_alphanumeric() || b"+/=_-\r\n".contains(&c)))
}

fn yq(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn wrapper(s: &Sub) -> String {
    format!(
        "mode: rule\nproxy-providers:\n  sub:\n    type: http\n    url: {}\n    path: ./providers/{}.txt\n    interval: 86400\n    header:\n      User-Agent: [{}]\n    health-check:\n      enable: true\n      url: http://www.gstatic.com/generate_204\n      interval: 600\nproxy-groups:\n  - name: PROXY\n    type: select\n    use: [sub]\nrules:\n  - MATCH,PROXY\n",
        yq(&s.url),
        safe(&s.name),
        yq(&s.ua)
    )
}

fn fetch(s: &Sub, proxy: Option<u16>) -> Result<(), String> {
    let r = net::get(&s.url, &s.ua, None).or_else(|e| match proxy {
        Some(p) => net::get(&s.url, &s.ua, Some(p)),
        None => Err(e),
    })?;
    let t = String::from_utf8_lossy(&r);
    let body = if is_config(&t) {
        t.into_owned()
    } else if is_links(&t) {
        wrapper(s)
    } else {
        return Err("Р С•РЎвЂљР Р†Р ВµРЎвЂљ Р Р…Р Вµ Р С—Р С•РЎвЂ¦Р С•Р В¶ Р Р…Р В° Р С—Р С•Р Т‘Р С—Р С‘РЎРѓР С”РЎС“".into());
    };
    fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
    fs::write(path(&file_of(&s.name)), body).map_err(|e| e.to_string())
}

pub fn update(name: &str, proxy: Option<u16>) -> Result<usize, String> {
    let subs: Vec<Sub> = subs().into_iter().filter(|s| name.is_empty() || s.name == name).collect();
    if subs.is_empty() {
        return Err("Р СњР ВµРЎвЂљ Р С—Р С•Р Т‘Р С—Р С‘РЎРѓР С•Р С” Р Т‘Р В»РЎРЏ Р С•Р В±Р Р…Р С•Р Р†Р В»Р ВµР Р…Р С‘РЎРЏ".into());
    }
    let mut errs = Vec::new();
    for s in &subs {
        if let Err(e) = fetch(s, proxy) {
            errs.push(format!("{}: {e}", s.name));
        }
    }
    if errs.is_empty() {
        Ok(subs.len())
    } else {
        Err(errs.join("\n"))
    }
}

pub fn inject(src: &str, apps: &[String]) -> String {
    if apps.is_empty() {
        return src.to_string();
    }
    let rules: Vec<String> = apps.iter().map(|a| format!("PROCESS-NAME,{a},DIRECT")).collect();
    let lines: Vec<&str> = src.lines().collect();
    let Some(i) = lines.iter().position(|l| l.starts_with("rules:")) else {
        let mut o = src.trim_end().to_string();
        o.push_str("\nrules:\n");
        for r in &rules {
            o.push_str(&format!("  - {r}\n"));
        }
        return o;
    };
    let mut o = String::with_capacity(src.len() + rules.len() * 40);
    for l in &lines[..i] {
        o.push_str(l);
        o.push('\n');
    }
    let head = lines[i];
    match head.find('[') {
        Some(p) => {
            let rest = head[p + 1..].trim_start();
            let sep = if rest.starts_with(']') { "" } else { ", " };
            let quoted: Vec<String> = rules.iter().map(|r| format!("\"{r}\"")).collect();
            o.push_str(&format!("rules: [{}{sep}{rest}\n", quoted.join(", ")));
        }
        None => {
            o.push_str(head);
            o.push('\n');
            let ind = lines[i + 1..]
                .iter()
                .find(|l| l.trim_start().starts_with('-'))
                .map(|l| &l[..l.len() - l.trim_start().len()])
                .unwrap_or("  ");
            for r in &rules {
                o.push_str(&format!("{ind}- {r}\n"));
            }
        }
    }
    for l in &lines[i + 1..] {
        o.push_str(l);
        o.push('\n');
    }
    o
}

const OWN: [&str; 6] = ["port:", "socks-port:", "mixed-port:", "redir-port:", "tproxy-port:", "find-process-mode:"];

const FAKE: &str = "fake-ip-range: 198.19.0.1/16";

fn own(src: &str, port: u16) -> String {
    let mut o = format!("mixed-port: {port}\nfind-process-mode: always\n");
    let has = src.contains("fake-ip-range:");
    let lines: Vec<&str> = src.lines().filter(|l| !OWN.iter().any(|k| l.starts_with(k))).collect();
    for (i, l) in lines.iter().enumerate() {
        let t = l.trim_start();
        if t.starts_with("fake-ip-range:") {
            o.push_str(&l[..l.len() - t.len()]);
            o.push_str(FAKE);
        } else {
            o.push_str(l);
        }
        o.push('\n');
        if !has && l.trim_end() == "dns:" {
            let next = lines.get(i + 1).copied().unwrap_or("");
            let ind = &next[..next.len() - next.trim_start().len()];
            o.push_str(&format!("{}{FAKE}\n", if ind.is_empty() { "  " } else { ind }));
        }
    }
    o
}

pub fn build(file: &str, apps: &[String], port: u16) -> Result<PathBuf, String> {
    let src = fs::read_to_string(path(file)).map_err(|e| format!("{file}: {e}"))?;
    fs::create_dir_all(work()).map_err(|e| e.to_string())?;
    let out = run_path();
    fs::write(&out, inject(&own(&src, port), apps)).map_err(|e| e.to_string())?;
    Ok(out)
}
