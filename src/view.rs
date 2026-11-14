use crate::json::V;
use crate::{Conn, Group, Node};
use slint::{ModelRc, SharedString, VecModel};
use std::collections::{BTreeSet, HashSet};

#[derive(Clone, PartialEq)]
pub struct PNode {
    pub name: String,
    pub kind: String,
    pub delay: i32,
    pub selected: bool,
}

#[derive(Clone, PartialEq)]
pub struct PGroup {
    pub name: String,
    pub kind: String,
    pub now: String,
    pub nodes: Vec<PNode>,
}

pub struct PConn {
    id: String,
    host: String,
    process: String,
    chain: String,
    rule: String,
    up: String,
    down: String,
}

pub struct Snap {
    pub down: u64,
    pub up: u64,
    pub conns: Vec<PConn>,
    pub apps: Vec<String>,
}

const KINDS: [&str; 3] = ["Selector", "URLTest", "Fallback"];

pub fn groups(v: &V, mode: &str) -> Vec<PGroup> {
    let px = v.get("proxies");
    let mut names: Vec<&str> = Vec::new();
    if mode == "global" {
        names.push("GLOBAL");
    }
    for n in px.get("GLOBAL").get("all").arr() {
        let n = n.str();
        if KINDS.contains(&px.get(n).get("type").str()) {
            names.push(n);
        }
    }
    names
        .into_iter()
        .map(|g| {
            let gv = px.get(g);
            let now = gv.get("now").str().to_string();
            let nodes = gv
                .get("all")
                .arr()
                .iter()
                .map(|n| {
                    let n = n.str();
                    let p = px.get(n);
                    let delay = p.get("history").arr().last().map_or(-1, |h| h.get("delay").num() as i32);
                    PNode { name: n.to_string(), kind: p.get("type").str().to_string(), delay, selected: n == now }
                })
                .collect();
            PGroup { name: g.to_string(), kind: gv.get("type").str().to_string(), now, nodes }
        })
        .collect()
}

pub fn resolve(groups: &[PGroup], node: &str) -> (String, i32) {
    let mut end = node.to_string();
    for _ in 0..8 {
        match groups.iter().find(|g| g.name == end) {
            Some(g) if !g.now.is_empty() => end = g.now.clone(),
            _ => break,
        }
    }
    let delay = groups.iter().flat_map(|g| &g.nodes).find(|n| n.name == end).map_or(-1, |n| n.delay);
    (end, delay)
}

impl Snap {
    pub fn from(v: &V) -> Self {
        let mut apps = BTreeSet::new();
        let conns = v
            .get("connections")
            .arr()
            .iter()
            .take(400)
            .map(|c| {
                let m = c.get("metadata");
                let host = match m.get("host").str() {
                    "" => m.get("destinationIP").str(),
                    h => h,
                };
                let process = m.get("process").str();
                if !process.is_empty() {
                    apps.insert(process.to_string());
                }
                let chain: Vec<&str> = c.get("chains").arr().iter().rev().map(V::str).collect();
                let rule = match c.get("rulePayload").str() {
                    "" => c.get("rule").str().to_string(),
                    p => format!("{} {p}", c.get("rule").str()),
                };
                PConn {
                    id: c.get("id").str().to_string(),
                    host: format!("{host}:{}", port(m.get("destinationPort"))),
                    process: if process.is_empty() { "—".into() } else { process.to_string() },
                    chain: chain.join(" → "),
                    rule,
                    up: size(c.get("upload").num()),
                    down: size(c.get("download").num()),
                }
            })
            .collect();
        Self {
            down: v.get("downloadTotal").num() as u64,
            up: v.get("uploadTotal").num() as u64,
            conns,
            apps: apps.into_iter().collect(),
        }
    }
}

fn port(v: &V) -> String {
    match v.str() {
        "" => format!("{}", v.num() as u32),
        s => s.to_string(),
    }
}

pub fn size(b: f64) -> String {
    let u = ["Б", "КБ", "МБ", "ГБ", "ТБ"];
    let mut v = b.max(0.0);
    let mut i = 0;
    while v >= 1024.0 && i < u.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{v:.0} {}", u[i])
    } else {
        format!("{v:.1} {}", u[i])
    }
}

pub fn rate(b: f64) -> String {
    format!("{}/с", size(b))
}

pub fn strings(v: &[String]) -> ModelRc<SharedString> {
    ModelRc::new(VecModel::from(v.iter().map(|s| SharedString::from(s.as_str())).collect::<Vec<_>>()))
}

pub fn to_groups(v: &[PGroup], open: &HashSet<String>, testing: &HashSet<String>) -> ModelRc<Group> {
    let gs: Vec<Group> = v
        .iter()
        .map(|g| {
            let nodes: Vec<Node> = g
                .nodes
                .iter()
                .map(|n| Node { name: n.name.as_str().into(), kind: n.kind.as_str().into(), delay: n.delay, selected: n.selected })
                .collect();
            Group {
                name: g.name.as_str().into(),
                kind: g.kind.as_str().into(),
                now: g.now.as_str().into(),
                nodes: ModelRc::new(VecModel::from(nodes)),
                open: open.contains(&g.name),
                testing: testing.contains(&g.name),
            }
        })
        .collect();
    ModelRc::new(VecModel::from(gs))
}

pub fn to_conns(v: Vec<PConn>) -> ModelRc<Conn> {
    let cs: Vec<Conn> = v
        .into_iter()
        .map(|c| Conn {
            id: c.id.into(),
            host: c.host.into(),
            process: c.process.into(),
            chain: c.chain.into(),
            rule: c.rule.into(),
            up: c.up.into(),
            down: c.down.into(),
        })
        .collect();
    ModelRc::new(VecModel::from(cs))
}
