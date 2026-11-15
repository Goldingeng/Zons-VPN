use crate::util::base;
use std::fs;
use std::path::PathBuf;

#[derive(Clone)]
pub struct Settings {
    pub profile: String,
    pub mode: String,
    pub port: u16,
    pub sysproxy: bool,
    pub tun: bool,
    pub bypass: Vec<String>,
    pub known: Vec<(String, String, String)>,
}

fn file() -> PathBuf {
    base().join("settings.ini")
}

impl Settings {
    pub fn load() -> Self {
        let mut s = Self {
            profile: String::new(),
            mode: "rule".into(),
            port: 7890,
            sysproxy: true,
            tun: false,
            bypass: Vec::new(),
            known: Vec::new(),
        };
        let Ok(t) = fs::read_to_string(file()) else { return s };
        for l in t.lines() {
            let Some((k, v)) = l.split_once('=') else { continue };
            let v = v.trim();
            match k.trim() {
                "profile" => s.profile = v.into(),
                "mode" if matches!(v, "rule" | "global" | "direct") => s.mode = v.into(),
                "port" => s.port = v.parse().unwrap_or(7890),
                "sysproxy" => s.sysproxy = v == "1",
                "tun" => s.tun = v == "1",
                "bypass" => s.bypass = v.split('|').filter(|a| !a.is_empty()).map(String::from).collect(),
                "known" => {
                    s.known = v
                        .split('|')
                        .filter_map(|e| {
                            let mut p = e.split('>');
                            Some((p.next()?.to_string(), p.next()?.to_string(), p.next().unwrap_or("").to_string()))
                        })
                        .filter(|k| !k.0.is_empty())
                        .collect()
                }
                _ => {}
            }
        }
        s
    }

    pub fn save(&self) {
        let _ = fs::write(
            file(),
            format!(
                "profile={}\nmode={}\nport={}\nsysproxy={}\ntun={}\nbypass={}\nknown={}\n",
                self.profile,
                self.mode,
                self.port,
                self.sysproxy as u8,
                self.tun as u8,
                self.bypass.join("|"),
                self.known.iter().map(|k| format!("{}>{}>{}", k.0, k.1, k.2)).collect::<Vec<_>>().join("|")
            ),
        );
    }
}
