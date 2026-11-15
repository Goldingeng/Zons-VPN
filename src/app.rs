use crate::api::Api;
use crate::engine::Core;
use crate::ffi::{IsUserAnAdmin, ShellExecuteW};
use crate::settings::Settings;
use crate::tray::{self, Ev};
use crate::util::{base, exe, near_port, port_open, w, wp, work};
use crate::view::{self, PGroup, Snap};
use crate::{autorun, coreup, dialog, profiles, sysproxy, AppWindow, Profile, State};
use slint::winit_030::winit::event::WindowEvent;
use slint::winit_030::{EventResult, WinitWindowAccessor};
use slint::{ComponentHandle, ModelRc, Timer, TimerMode, VecModel, Weak};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashSet};
use std::path::Path;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

struct Ctl {
    ui: Weak<AppWindow>,
    core: Core,
    st: Settings,
    admin: bool,
    ok: bool,
    gen: usize,
    open: HashSet<String>,
    seeded: bool,
    testing: HashSet<String>,
    groups: Vec<PGroup>,
    seen: BTreeSet<String>,
    last: Option<(u64, u64, Instant)>,
    busy: Arc<AtomicBool>,
    dirty: bool,
    tick: u32,
    updating: bool,
    down: bool,
    fresh: bool,
    fetching: bool,
    _timer: Timer,
}

thread_local! {
    static CTL: RefCell<Option<Ctl>> = const { RefCell::new(None) };
}

fn with<R>(f: impl FnOnce(&mut Ctl) -> R) -> Option<R> {
    CTL.with(|c| c.try_borrow_mut().ok()?.as_mut().map(f))
}

fn later(f: impl FnOnce(&mut Ctl) + Send + 'static) {
    let _ = slint::invoke_from_event_loop(move || {
        with(f);
    });
}

pub fn init(ui: &AppWindow) {
    let st = Settings::load();
    let admin = unsafe { IsUserAnAdmin() } != 0;
    let s = ui.global::<State>();
    s.set_admin(admin);
    s.set_sysproxy(st.sysproxy);
    s.set_tun(st.tun && admin);
    s.set_mode(st.mode.as_str().into());
    s.set_port(st.port as i32);
    s.set_autostart(autorun::get());
    bind(&s);
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_secs(1), || {
        with(Ctl::poll);
    });
    let ctl = Ctl {
        ui: ui.as_weak(),
        core: Core::new(),
        st,
        admin,
        ok: false,
        gen: 0,
        open: HashSet::new(),
        seeded: false,
        testing: HashSet::new(),
        groups: Vec::new(),
        seen: BTreeSet::new(),
        last: None,
        busy: Arc::new(AtomicBool::new(false)),
        dirty: false,
        tick: 0,
        updating: false,
        down: false,
        fresh: false,
        fetching: false,
        _timer: timer,
    };
    CTL.with(|c| *c.borrow_mut() = Some(ctl));
    with(Ctl::boot);
}

fn bind(s: &State) {
    s.on_page_changed(|_| {
        with(|c| {
            c.dirty = true;
            c.poll();
        });
    });
    s.on_set_sysproxy(|v| {
        with(|c| c.set_sys(v));
    });
    s.on_set_tun(|v| {
        if with(|c| c.set_tun(v)) == Some(true) {
            elevate();
        }
    });
    s.on_set_mode(|m| {
        with(|c| c.set_mode(m.as_str()));
    });
    s.on_select(|g, n| {
        with(|c| c.select(g.to_string(), n.to_string()));
    });
    s.on_test(|g| {
        with(|c| c.test(g.to_string()));
    });
    s.on_toggle_group(|g| {
        with(|c| c.toggle(g.as_str()));
    });
    s.on_use_profile(|f| {
        with(|c| c.use_profile(f.to_string()));
    });
    s.on_add_profile(|n, u| {
        with(|c| c.add_profile(n.as_str(), u.as_str()));
    });
    s.on_remove_profile(|f| {
        with(|c| c.remove_profile(f.as_str()));
    });
    s.on_update_profiles(|n| {
        with(|c| c.update(n.to_string()));
    });
    s.on_add_app(|a| {
        with(|c| c.add_app(a.as_str()));
    });
    s.on_remove_app(|a| {
        with(|c| c.remove_app(a.as_str()));
    });
    s.on_browse_app(|| {
        if let Some(a) = dialog::pick_exe() {
            with(|c| c.add_app(&a));
        }
    });
    s.on_close_conn(|id| {
        with(|c| c.close(Some(id.to_string())));
    });
    s.on_close_all(|| {
        with(|c| c.close(None));
    });
    s.on_set_port(|p| {
        with(|c| c.set_port(p));
    });
    s.on_set_autostart(autorun::set);
    s.on_restart(|| {
        with(Ctl::start);
    });
    s.on_update_core(|| {
        with(|c| c.fetch_core(true));
    });
    s.on_open_dir(|| open(&base()));
    s.on_open_log(|| open(&work().join("mihomo.log")));
    s.on_quit(quit);
}

impl Ctl {
    fn ui(&self, f: impl FnOnce(&State)) {
        if let Some(u) = self.ui.upgrade() {
            f(&u.global::<State>());
        }
    }

    fn api(&self) -> Api {
        self.core.api()
    }

    fn say(&self, msg: &str, err: bool) {
        self.ui(|s| {
            s.set_status(msg.into());
            s.set_status_bad(err);
        });
        if err {
            tray::notify("Zons VPN", msg, true);
        }
    }

    fn boot(&mut self) {
        let _ = std::fs::create_dir_all(profiles::dir());
        self.show_bypass();
        self.pick();
        self.show_profiles();
        if Core::exe().is_none() && !coreup::staged() {
            self.fetch_core(true);
        } else if !self.st.profile.is_empty() {
            self.start();
        } else if !profiles::subs().is_empty() {
            self.update(String::new());
        }
        self.sync();
    }

    fn pick(&mut self) {
        let l = profiles::list();
        if !l.contains(&self.st.profile) {
            self.st.profile = l.first().cloned().unwrap_or_default();
            self.st.save();
        }
    }

    fn start(&mut self) {
        self.halt();
        self.gen += 1;
        if coreup::apply() {
            self.fresh = true;
        }
        self.show_core();
        if self.st.profile.is_empty() {
            self.sync();
            return;
        }
        let port = near_port(self.st.port);
        if port != self.st.port {
            self.say(&format!("Порт {} занят, прокси работает на {port}", self.st.port), false);
            self.st.port = port;
            self.st.save();
            self.ui(|s| s.set_port(port as i32));
        }
        let r = match profiles::build(&self.st.profile, &self.st.bypass, self.st.port) {
            Ok(p) => self.core.start(&p),
            Err(e) => return self.fail(&e),
        };
        if let Err(e) = r {
            if self.revert() {
                return;
            }
            self.fail(&e);
            return;
        }
        let name = profiles::label(&self.st.profile);
        self.ui(|s| {
            s.set_running(true);
            s.set_ready(false);
            s.set_profile(name.as_str().into());
        });
        self.launch(false);
        self.sync();
    }

    fn launch(&mut self, reload: bool) {
        let (api, st, g) = (self.api(), self.st.clone(), self.gen);
        let tun = st.tun && self.admin;
        let run = profiles::run_path().display().to_string();
        thread::spawn(move || {
            let up = if reload { api.reload(&run) } else { api.ready(10_000) };
            let ok = up && api.setup(st.port, &st.mode, tun);
            if ok && st.sysproxy {
                sysproxy::set(Some(st.port));
            }
            later(move |c| c.started(g, ok));
        });
    }

    fn started(&mut self, g: usize, ok: bool) {
        if g != self.gen {
            return;
        }
        if self.fresh {
            if !ok && self.revert() {
                return;
            }
            self.fresh = false;
            coreup::commit();
        }
        self.ok = ok;
        if !ok {
            if self.st.sysproxy {
                sysproxy::set(None);
            }
            self.say("Ядро не отвечает. Подробности: Настройки → Журнал ядра", true);
        }
        let alive = self.core.alive();
        self.ui(|s| {
            s.set_ready(ok);
            s.set_running(alive);
        });
        self.dirty = true;
        self.poll();
        self.sync();
    }

    fn fail(&mut self, e: &str) {
        if self.st.sysproxy {
            sysproxy::set(None);
        }
        self.ui(|s| {
            s.set_running(false);
            s.set_ready(false);
        });
        self.say(e, true);
        self.sync();
    }

    fn halt(&mut self) {
        if self.ok && self.st.tun && self.admin {
            self.api().tun(false);
        }
        self.ok = false;
        self.last = None;
        self.core.stop();
        self.ui(|s| {
            s.set_ready(false);
            s.set_running(false);
        });
    }

    fn reload(&mut self) {
        if self.st.profile.is_empty() {
            return;
        }
        if !self.ok || !self.core.alive() {
            return self.start();
        }
        match profiles::build(&self.st.profile, &self.st.bypass, self.st.port) {
            Ok(_) => {
                self.gen += 1;
                self.launch(true);
            }
            Err(e) => self.say(&e, true),
        }
    }

    fn sync(&self) {
        let alive = self.core.alive();
        let tun = self.st.tun && self.admin;
        let i = if !alive {
            0
        } else if self.ok && (self.st.sysproxy || tun) {
            2
        } else {
            1
        };
        let mut tip = format!("Zons VPN · {}", profiles::label(&self.st.profile));
        if alive && self.st.sysproxy {
            tip.push_str(" · прокси");
        }
        if alive && tun {
            tip.push_str(" · TUN");
        }
        tray::set(i, &tip);
    }

    fn set_sys(&mut self, v: bool) {
        self.st.sysproxy = v;
        self.st.save();
        sysproxy::set((v && self.ok).then_some(self.st.port));
        self.ui(|s| s.set_sysproxy(v));
        self.sync();
    }

    fn set_tun(&mut self, v: bool) -> bool {
        if v && !self.admin {
            self.st.tun = true;
            self.st.save();
            return true;
        }
        self.st.tun = v;
        self.st.save();
        self.ui(|s| s.set_tun(v));
        if self.ok {
            let api = self.api();
            thread::spawn(move || {
                api.tun(v);
            });
        }
        self.sync();
        false
    }

    fn set_mode(&mut self, m: &str) {
        if !matches!(m, "rule" | "global" | "direct") {
            return;
        }
        if !self.ok || self.api().patch(&format!("{{\"mode\":\"{m}\"}}")) {
            self.st.mode = m.into();
            self.st.save();
        }
        let cur = self.st.mode.clone();
        self.ui(|s| s.set_mode(cur.as_str().into()));
        self.dirty = true;
        self.poll();
    }

    fn set_port(&mut self, p: i32) {
        if (1..=65535).contains(&p) && p as u16 != self.st.port && !port_open(p as u16) {
            self.say(&format!("Порт {p} занят другой программой"), true);
        } else if (1..=65535).contains(&p) {
            self.st.port = p as u16;
            self.st.save();
            if self.ok {
                let (api, port, sys) = (self.api(), self.st.port, self.st.sysproxy);
                thread::spawn(move || {
                    if api.patch(&format!("{{\"mixed-port\":{port}}}")) && sys {
                        sysproxy::set(Some(port));
                    }
                });
            }
        }
        let port = self.st.port as i32;
        self.ui(|s| s.set_port(port));
    }

    fn show_core(&self) {
        let ui = self.ui.clone();
        thread::spawn(move || {
            let v = coreup::current();
            let _ = ui.upgrade_in_event_loop(move |u| u.global::<State>().set_core_version(v.as_str().into()));
        });
    }

    fn fetch_core(&mut self, force: bool) {
        if self.fetching {
            return;
        }
        self.fetching = true;
        if force {
            self.say("Проверяю обновление ядра mihomo…", false);
        }
        let proxy = self.ok.then_some(self.st.port);
        thread::spawn(move || {
            let r = coreup::check(proxy, force);
            later(move |c| c.fetched(force, r));
        });
    }

    fn fetched(&mut self, force: bool, r: Result<Option<String>, String>) {
        self.fetching = false;
        let first = Core::exe().is_none();
        match r {
            Ok(Some(tag)) if first || !self.core.alive() => {
                self.say(&format!("Ядро mihomo {tag} установлено"), false);
                self.start();
            }
            Ok(Some(tag)) => self.say(&format!("Ядро mihomo {tag} скачано, установится при следующем запуске"), false),
            Ok(None) if force && !first => self.say("Установлена последняя версия ядра", false),
            Ok(None) => {}
            Err(e) if first || force => self.say(&format!("Не удалось скачать ядро mihomo: {e}"), true),
            Err(_) => {}
        }
    }

    fn revert(&mut self) -> bool {
        if !self.fresh {
            return false;
        }
        self.fresh = false;
        self.halt();
        if !coreup::rollback() {
            return false;
        }
        self.say("Новое ядро не запустилось, вернул прежнее", true);
        self.start();
        true
    }

    fn render(&self) {
        let m = view::to_groups(&self.groups, &self.open, &self.testing);
        let (node, grp) = self.groups.first().map(|g| (g.now.clone(), g.name.clone())).unwrap_or_default();
        let (end, delay) = view::resolve(&self.groups, &node);
        self.ui(|s| {
            s.set_groups(m);
            s.set_node(node.as_str().into());
            s.set_node_group(grp.as_str().into());
            s.set_node_end(end.as_str().into());
            s.set_node_delay(delay);
        });
    }

    fn select(&mut self, g: String, n: String) {
        if !self.ok {
            return;
        }
        for grp in self.groups.iter_mut().filter(|x| x.name == g) {
            grp.now = n.clone();
            for nd in &mut grp.nodes {
                nd.selected = nd.name == n;
            }
        }
        self.render();
        let api = self.api();
        thread::spawn(move || {
            api.select(&g, &n);
            later(|c| {
                c.dirty = true;
                c.poll();
            });
        });
    }

    fn test(&mut self, g: String) {
        if !self.ok || !self.testing.insert(g.clone()) {
            return;
        }
        self.render();
        let api = self.api();
        thread::spawn(move || {
            api.test(&g);
            later(move |c| {
                c.testing.remove(&g);
                c.render();
                c.dirty = true;
                c.poll();
            });
        });
    }

    fn toggle(&mut self, g: &str) {
        if !self.open.remove(g) {
            self.open.insert(g.to_string());
        }
        self.render();
    }

    fn close(&mut self, id: Option<String>) {
        if !self.ok {
            return;
        }
        let api = self.api();
        thread::spawn(move || {
            match id {
                Some(i) => api.close(&i),
                None => api.close_all(),
            };
            later(Ctl::poll);
        });
    }

    fn poll(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        if self.ok && !self.core.alive() {
            self.ok = false;
            self.fail("Ядро неожиданно завершилось. Подробности: Настройки → Журнал ядра");
        }
        if !self.ok {
            self.ui(|s| {
                s.set_down_speed("0 Б/с".into());
                s.set_up_speed("0 Б/с".into());
            });
            return;
        }
        if self.tick % 3600 == 10 {
            self.fetch_core(false);
        }
        if self.busy.swap(true, Ordering::SeqCst) {
            return;
        }
        let px = self.dirty || self.tick % 5 == 1;
        self.dirty = false;
        let (api, busy, g, mode) = (self.api(), self.busy.clone(), self.gen, self.st.mode.clone());
        thread::spawn(move || {
            let groups = if px { api.json("/proxies").map(|v| view::groups(&v, &mode)) } else { None };
            let snap = api.json("/connections").map(|v| Snap::from(&v));
            busy.store(false, Ordering::SeqCst);
            later(move |c| c.polled(g, groups, snap));
        });
    }

    fn polled(&mut self, g: usize, groups: Option<Vec<PGroup>>, snap: Option<Snap>) {
        if g != self.gen || !self.ok {
            return;
        }
        if let Some(gs) = groups {
            if !self.seeded {
                if let Some(f) = gs.first() {
                    self.open.insert(f.name.clone());
                    self.seeded = true;
                }
            }
            if gs != self.groups {
                self.groups = gs;
                self.render();
            }
        }
        let Some(sn) = snap else { return };
        let now = Instant::now();
        let (ds, us) = match self.last {
            Some((d, u, t)) => {
                let s = now.duration_since(t).as_secs_f64().max(0.001);
                (sn.down.saturating_sub(d) as f64 / s, sn.up.saturating_sub(u) as f64 / s)
            }
            None => (0.0, 0.0),
        };
        self.last = Some((sn.down, sn.up, now));
        self.seen.extend(sn.apps);
        let apps: Vec<String> = self.seen.iter().cloned().filter(|a| !self.st.bypass.iter().any(|b| b.eq_ignore_ascii_case(a))).collect();
        let on_conns = self.ui.upgrade().is_some_and(|u| u.global::<State>().get_page() == 4);
        let conns = on_conns.then(|| view::to_conns(sn.conns));
        let (down, up) = (sn.down as f64, sn.up as f64);
        self.ui(|s| {
            s.set_down_speed(view::rate(ds).into());
            s.set_up_speed(view::rate(us).into());
            s.set_down_total(view::size(down).into());
            s.set_up_total(view::size(up).into());
            s.set_seen_apps(view::strings(&apps));
            if let Some(m) = conns {
                s.set_conns(m);
            }
        });
    }

    fn show_profiles(&self) {
        let subs = profiles::subs();
        let mut files = profiles::list();
        for s in &subs {
            let f = profiles::file_of(&s.name);
            if !files.contains(&f) {
                files.push(f);
            }
        }
        let v: Vec<Profile> = files
            .iter()
            .map(|f| {
                let url = subs.iter().find(|s| profiles::file_of(&s.name) == *f).map(|s| s.url.clone()).unwrap_or_default();
                Profile { name: profiles::label(f).into(), file: f.as_str().into(), url: url.into(), active: *f == self.st.profile }
            })
            .collect();
        self.ui(|s| s.set_profiles(ModelRc::new(VecModel::from(v))));
    }

    fn use_profile(&mut self, f: String) {
        if f == self.st.profile && self.core.alive() {
            return;
        }
        self.st.profile = f;
        self.st.save();
        self.show_profiles();
        self.start();
    }

    fn add_profile(&mut self, n: &str, u: &str) {
        match profiles::add(n, u) {
            Ok(name) => {
                self.show_profiles();
                self.update(name);
            }
            Err(e) => self.say(&e, true),
        }
    }

    fn remove_profile(&mut self, f: &str) {
        if let Err(e) = profiles::remove(f) {
            self.say(&e, true);
        }
        if f == self.st.profile {
            self.halt();
            self.st.profile.clear();
            self.pick();
            self.start();
        }
        self.show_profiles();
    }

    fn update(&mut self, name: String) {
        if self.updating {
            return;
        }
        self.updating = true;
        self.ui(|s| {
            s.set_updating(true);
            s.set_status("Обновление подписок…".into());
            s.set_status_bad(false);
        });
        let proxy = self.ok.then_some(self.st.port);
        thread::spawn(move || {
            let r = profiles::update(&name, proxy);
            later(move |c| c.updated(name, r));
        });
    }

    fn updated(&mut self, name: String, r: Result<usize, String>) {
        self.updating = false;
        self.ui(|s| s.set_updating(false));
        match &r {
            Ok(n) => self.say(&format!("Обновлено подписок: {n}"), false),
            Err(e) => self.say(e, true),
        }
        let before = self.st.profile.clone();
        self.pick();
        self.show_profiles();
        if self.st.profile.is_empty() {
            return;
        }
        if name.is_empty() || profiles::file_of(&name) == self.st.profile || before != self.st.profile || !self.core.alive() {
            self.reload();
        }
    }

    fn show_bypass(&self) {
        let v = view::strings(&self.st.bypass);
        self.ui(|s| s.set_bypass(v));
    }

    fn add_app(&mut self, a: &str) {
        let a = a.trim().trim_matches('"');
        let a = a.rsplit(['\\', '/']).next().unwrap_or("");
        let a: String = a.chars().filter(|c| !matches!(c, ',' | '"' | '\'' | '\n' | '\r' | '|')).collect();
        let a = a.trim();
        if a.is_empty() {
            return;
        }
        let a = if a.contains('.') { a.to_string() } else { format!("{a}.exe") };
        if self.st.bypass.iter().any(|b| b.eq_ignore_ascii_case(&a)) {
            return;
        }
        self.st.bypass.push(a);
        self.st.bypass.sort_by_key(|s| s.to_lowercase());
        self.st.save();
        self.show_bypass();
        self.reload();
    }

    fn remove_app(&mut self, a: &str) {
        self.st.bypass.retain(|b| b != a);
        self.st.save();
        self.show_bypass();
        self.reload();
    }

    fn shutdown(&mut self) {
        if self.down {
            return;
        }
        self.down = true;
        if self.st.sysproxy {
            sysproxy::set(None);
        }
        self.halt();
    }
}

fn open(p: &Path) {
    unsafe { ShellExecuteW(null_mut(), w("open").as_ptr(), wp(p).as_ptr(), null(), null(), 1) };
}

fn elevate() {
    let r = unsafe { ShellExecuteW(null_mut(), w("runas").as_ptr(), wp(&exe()).as_ptr(), w("--wait").as_ptr(), wp(&base()).as_ptr(), 1) } as isize;
    if r > 32 {
        quit();
    } else {
        with(|c| {
            c.st.tun = false;
            c.st.save();
            c.ui(|s| s.set_tun(false));
        });
    }
}

fn show() {
    if let Some(u) = with(|c| c.ui.upgrade()).flatten() {
        let _ = u.show();
        u.window().set_minimized(false);
        u.set_flip(!u.get_flip());
    }
}

pub fn repaint(ui: &AppWindow) {
    let weak = ui.as_weak();
    ui.window().on_winit_window_event(move |_, e| {
        let full = match e {
            WindowEvent::Resized(s) => s.width > 0 && s.height > 0,
            WindowEvent::Occluded(o) => !o,
            _ => false,
        };
        if let (true, Some(u)) = (full, weak.upgrade()) {
            u.set_flip(!u.get_flip());
        }
        EventResult::Propagate
    });
}

fn quit() {
    shutdown();
    let _ = slint::quit_event_loop();
}

pub fn shutdown() {
    with(Ctl::shutdown);
    tray::remove();
}

pub fn on_tray(e: Ev) {
    let _ = slint::invoke_from_event_loop(move || match e {
        Ev::Show => show(),
        Ev::Sys => {
            with(|c| {
                let v = !c.st.sysproxy;
                c.set_sys(v);
            });
        }
        Ev::Tun => {
            if with(|c| {
                let v = !(c.st.tun && c.admin);
                c.set_tun(v)
            }) == Some(true)
            {
                elevate();
            }
        }
        Ev::Quit => quit(),
    });
}

pub fn tray_state() -> (bool, bool) {
    with(|c| (c.st.sysproxy, c.st.tun && c.admin)).unwrap_or_default()
}
