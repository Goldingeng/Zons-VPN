use crate::menu::{Act, Menu};
use crate::tray::{boxed, post, App, Side, WM_TESTED};
use crate::util::{base, work};
use crate::{autorun, profiles, sysproxy};

impl App {
    pub fn menu(&mut self) -> Menu {
        let mut m = Menu::new();
        let h = m.h;
        let alive = self.core.alive();
        Menu::label(h, &if alive { format!("mihomo · {}", profiles::label(&self.st.profile)) } else { "Ядро остановлено".into() });
        Menu::sep(h);
        if alive {
            let mm = Menu::sub();
            for (k, t) in [("rule", "Правила"), ("global", "Глобально"), ("direct", "Напрямую")] {
                m.item(mm, Act::Mode(k), t, self.st.mode == k, true);
            }
            Menu::popup(h, mm, "Режим");
            if let Some(v) = self.core.api().json("/proxies") {
                m.groups(&v, &self.st.mode);
            }
            Menu::sep(h);
        }
        m.item(h, Act::Sys, "Системный прокси", self.st.sysproxy, true);
        m.item(h, Act::Tun, if self.admin { "TUN" } else { "TUN (перезапуск от админа)" }, self.st.tun && self.admin, true);
        Menu::sep(h);
        let pm = Menu::sub();
        for p in profiles::list() {
            let l = profiles::label(&p);
            let on = p == self.st.profile;
            m.item(pm, Act::Profile(p), &l, on, true);
        }
        Menu::sep(pm);
        m.item(pm, Act::Update, if self.busy { "Обновление…" } else { "Обновить подписки" }, false, !self.busy);
        m.item(pm, Act::Subs, "Редактировать подписки", false, true);
        Menu::popup(h, pm, "Профили");
        let om = Menu::sub();
        m.item(om, Act::Restart, "Перезапустить ядро", false, true);
        m.item(om, Act::Log, "Журнал ядра", false, true);
        m.item(om, Act::Dir, "Открыть папку", false, true);
        m.item(om, Act::Auto, "Автозапуск", autorun::get(), true);
        Menu::popup(h, om, "Ещё");
        Menu::sep(h);
        m.item(h, Act::Quit, "Выход", false, true);
        m
    }

    pub fn run(&mut self, act: Act) -> Option<Side> {
        match act {
            Act::Mode(k) => {
                if self.core.api().patch(&format!("{{\"mode\":\"{k}\"}}")) {
                    self.st.mode = k.into();
                    self.st.save();
                }
            }
            Act::Pick(g, n) => {
                self.core.api().select(&g, &n);
            }
            Act::Test(g) => {
                let api = self.core.api();
                let h = self.hwnd as usize;
                std::thread::spawn(move || {
                    let ok = api.test(&g);
                    post(h, WM_TESTED, ok as usize, boxed(g));
                });
            }
            Act::Profile(p) => {
                self.st.profile = p;
                self.st.save();
                self.start();
            }
            Act::Sys => {
                self.st.sysproxy = !self.st.sysproxy;
                self.st.save();
                sysproxy::set((self.st.sysproxy && self.ok).then_some(self.st.port));
                self.refresh();
            }
            Act::Tun => {
                if !self.admin {
                    self.st.tun = true;
                    self.st.save();
                    return Some(Side::Elevate);
                }
                self.st.tun = !self.st.tun;
                self.st.save();
                if self.ok {
                    self.core.api().tun(self.st.tun);
                }
                self.refresh();
            }
            Act::Update => self.update(),
            Act::Subs => return Some(Side::Open(profiles::subs_file())),
            Act::Dir => return Some(Side::Open(base())),
            Act::Log => return Some(Side::Open(work().join("mihomo.log"))),
            Act::Restart => self.start(),
            Act::Auto => autorun::set(!autorun::get()),
            Act::Quit => {
                self.shutdown();
                return Some(Side::Quit);
            }
        }
        None
    }

}
