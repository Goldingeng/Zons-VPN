use crate::ffi::*;
use crate::json::V;
use crate::util::w;

pub enum Act {
    Mode(&'static str),
    Pick(String, String),
    Test(String),
    Profile(String),
    Sys,
    Tun,
    Update,
    Subs,
    Dir,
    Log,
    Restart,
    Auto,
    Quit,
}

pub struct Menu {
    pub h: HMENU,
    pub acts: Vec<Act>,
}

impl Menu {
    pub fn new() -> Self {
        Self { h: unsafe { CreatePopupMenu() }, acts: Vec::new() }
    }

    pub fn sub() -> HMENU {
        unsafe { CreatePopupMenu() }
    }

    pub fn item(&mut self, m: HMENU, act: Act, text: &str, checked: bool, enabled: bool) {
        self.acts.push(act);
        let f = MF_STRING | if checked { MF_CHECKED } else { 0 } | if enabled { 0 } else { MF_GRAYED };
        unsafe { AppendMenuW(m, f, self.acts.len(), w(&text.replace('&', "&&")).as_ptr()) };
    }

    pub fn label(m: HMENU, text: &str) {
        unsafe { AppendMenuW(m, MF_STRING | MF_GRAYED, 0, w(&text.replace('&', "&&")).as_ptr()) };
    }

    pub fn popup(m: HMENU, sub: HMENU, text: &str) {
        unsafe { AppendMenuW(m, MF_POPUP, sub as usize, w(&text.replace('&', "&&")).as_ptr()) };
    }

    pub fn sep(m: HMENU) {
        unsafe { AppendMenuW(m, MF_SEPARATOR, 0, std::ptr::null()) };
    }

    pub fn take(self, id: i32) -> Option<Act> {
        unsafe { DestroyMenu(self.h) };
        if id <= 0 {
            return None;
        }
        self.acts.into_iter().nth(id as usize - 1)
    }

    pub fn groups(&mut self, v: &V, mode: &str) {
        let px = v.get("proxies");
        let mut names: Vec<&str> = Vec::new();
        if mode == "global" {
            names.push("GLOBAL");
        }
        for n in px.get("GLOBAL").get("all").arr() {
            let n = n.str();
            if matches!(px.get(n).get("type").str(), "Selector" | "URLTest" | "Fallback") {
                names.push(n);
            }
        }
        for g in names {
            let gv = px.get(g);
            let now = gv.get("now").str();
            let sm = Self::sub();
            self.item(sm, Act::Test(g.into()), "Проверить задержку", false, true);
            Self::sep(sm);
            for n in gv.get("all").arr() {
                let n = n.str();
                let label = match px.get(n).get("history").arr().last().map(|h| h.get("delay").num() as u32) {
                    Some(0) => format!("{n}\t×"),
                    Some(d) => format!("{n}\t{d} ms"),
                    None => n.to_string(),
                };
                self.item(sm, Act::Pick(g.into(), n.into()), &label, n == now, true);
            }
            Self::popup(self.h, sm, &format!("{g}\t{now}"));
        }
    }
}
