use crate::json::{self, V};
use crate::util::{jstr, pct};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::thread::sleep;
use std::time::{Duration, Instant};

const PROBE: &str = "http://www.gstatic.com/generate_204";

#[derive(Clone)]
pub struct Api {
    pub port: u16,
    pub secret: String,
}

impl Api {
    fn req(&self, m: &str, path: &str, body: &str, secs: u64) -> Option<(u16, String)> {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
        let mut s = TcpStream::connect_timeout(&addr, Duration::from_secs(2)).ok()?;
        s.set_read_timeout(Some(Duration::from_secs(secs))).ok()?;
        let r = format!(
            "{m} {path} HTTP/1.0\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            self.secret,
            body.len()
        );
        s.write_all(r.as_bytes()).ok()?;
        let mut buf = Vec::new();
        s.read_to_end(&mut buf).ok()?;
        let t = String::from_utf8_lossy(&buf);
        let (head, rest) = t.split_once("\r\n\r\n")?;
        let code = head.split(' ').nth(1)?.parse().ok()?;
        Some((code, rest.to_string()))
    }

    fn ok(&self, m: &str, path: &str, body: &str, secs: u64) -> bool {
        matches!(self.req(m, path, body, secs), Some((c, _)) if c / 100 == 2)
    }

    pub fn json(&self, path: &str) -> Option<V> {
        match self.req("GET", path, "", 5)? {
            (200, b) => json::parse(&b),
            _ => None,
        }
    }

    pub fn ready(&self, ms: u128) -> bool {
        let t = Instant::now();
        while t.elapsed().as_millis() < ms {
            if self.ok("GET", "/version", "", 2) {
                return true;
            }
            sleep(Duration::from_millis(200));
        }
        false
    }

    pub fn patch(&self, body: &str) -> bool {
        self.ok("PATCH", "/configs", body, 10)
    }

    pub fn setup(&self, port: u16, mode: &str, tun: bool) -> bool {
        let t = if tun { format!(",{}", tun_json(true)) } else { String::new() };
        self.patch(&format!("{{\"mixed-port\":{port},\"mode\":{}{t}}}", jstr(mode)))
    }

    pub fn tun(&self, on: bool) -> bool {
        self.patch(&format!("{{{}}}", tun_json(on)))
    }

    pub fn select(&self, group: &str, name: &str) -> bool {
        self.ok("PUT", &format!("/proxies/{}", pct(group)), &format!("{{\"name\":{}}}", jstr(name)), 5)
    }

    pub fn reload(&self, path: &str) -> bool {
        self.ok("PUT", "/configs?force=true", &format!("{{\"path\":{}}}", jstr(path)), 30)
    }

    pub fn close(&self, id: &str) -> bool {
        self.ok("DELETE", &format!("/connections/{}", pct(id)), "", 5)
    }

    pub fn close_all(&self) -> bool {
        self.ok("DELETE", "/connections", "", 5)
    }

    pub fn test(&self, group: &str) -> bool {
        self.ok("GET", &format!("/group/{}/delay?url={}&timeout=5000", pct(group), pct(PROBE)), "", 60)
    }
}

fn tun_json(on: bool) -> String {
    if on {
        "\"tun\":{\"enable\":true,\"device\":\"ZonsVPN\",\"stack\":\"mixed\",\"auto-route\":true,\"auto-detect-interface\":true,\"dns-hijack\":[\"any:53\"]}".into()
    } else {
        "\"tun\":{\"enable\":false}".into()
    }
}
