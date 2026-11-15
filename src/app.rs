use crate::api::Api;
use crate::engine::Core;
use crate::ffi::{IsUserAnAdmin, ShellExecuteW};
use crate::settings::Settings;
use crate::tray::{self, Ev};
use crate::util::{base, exe, near_port, port_open, w, wp, work};
use crate::view::{self, PGroup, Snap};
use crate::icon::{self, Px};
use crate::{autorun, coreup, dialog, pick, profiles, sysproxy, AppWindow, Pick, Profile, State};
use slint::winit_030::winit::event::WindowEvent;
use slint::winit_030::{EventResult, WinitWindowAccessor};
use slint::{ComponentHandle, Image, ModelRc, Timer, TimerMode, VecModel, Weak};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
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
    seen: BTreeMap<String, String>,
    last: Option<(u64, u64, Instant)>,
    busy: Arc<AtomicBool>,
    dirty: bool,
    tick: u32,
    updating: bool,
    down: bool,
    fresh: bool,
    fetching: bool,
    note: Timer,
    picks: Vec<(String, String, String)>,
    icons: HashMap<String, Image>,
    titles: HashMap<String, String>,
    query: String,
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
    s.set_side(st.side as f32);
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
        seen: BTreeMap::new(),
        last: None,
        busy: Arc::new(AtomicBool::new(false)),
        dirty: false,
        tick: 0,
        updating: false,
        down: false,
        fresh: false,
        fetching: false,
        note: Timer::default(),
        picks: Vec::new(),
        icons: HashMap::new(),
        titles: HashMap::new(),
        query: String::new(),
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
    s.on_side_changed(|w| {
        with(|c| {
            c.st.side = w.round() as u16;
            c.st.save();
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
    s.on_open_picks(|k| {
        with(|c| c.open_picks(k));
    });
    s.on_find_picks(|q| {
        with(|c| c.find_picks(q.to_string()));
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

    fn note(&self, msg: &str, err: bool) {
        self.ui(|s| {
            s.set_note(msg.into());
            s.set_note_bad(err);
        });
        let ui = self.ui.clone();
        self.note.start(TimerMode::SingleShot, Duration::from_secs(5), move || {
            if let Some(u) = ui.upgrade() {
                u.global::<State>().set_note("".into());
            }
        });
    }

    fn boot(&mut self) {
        let _ = std::fs::create_dir_all(profiles::dir());
        self.show_bypass();
        self.backfill();
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
        self.st.mode = m.into();
        self.st.save();
        self.ui(|s| s.set_mode(m.into()));
        if !self.ok {
            return;
        }
        let (api, g, body) = (self.api(), self.gen, format!("{{\"mode\":\"{m}\"}}"));
        thread::spawn(move || {
            if api.patch(&body) {
                api.close_all();
            }
            later(move |c| {
                if g == c.gen {
                    c.dirty = true;
                    c.poll();
                }
            });
        });
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
        for (n, p) in sn.apps {
            let slot = self.seen.entry(n).or_default();
            if slot.is_empty() {
                *slot = p;
            }
        }
        let paths: Vec<String> = self.seen.values().cloned().collect();
        self.fetch(paths);
        let on_conns = self.ui.upgrade().is_some_and(|u| u.global::<State>().get_page() == 4);
        let conns = on_conns.then(|| view::to_conns(sn.conns));
        let (down, up) = (sn.down as f64, sn.up as f64);
        self.ui(|s| {
            s.set_down_speed(view::rate(ds).into());
            s.set_up_speed(view::rate(us).into());
            s.set_down_total(view::size(down).into());
            s.set_up_total(view::size(up).into());
            if let Some(m) = conns {
                s.set_conns(m);
            }
        });
        self.show_seen();
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
            Err(e) => self.note(&e, true),
        }
    }

    fn remove_profile(&mut self, f: &str) {
        if let Err(e) = profiles::remove(f) {
            self.note(&e, true);
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
            s.set_note("Обновление подписок…".into());
            s.set_note_bad(false);
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
            Ok(n) => self.note(&format!("Обновлено подписок: {n}"), false),
            Err(e) => self.note(e, true),
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

    fn show_bypass(&mut self) {
        let paths: Vec<String> = self.st.known.iter().map(|k| k.1.clone()).collect();
        self.fetch(paths);
        self.show_lists();
    }

    fn backfill(&self) {
        let lost = |c: &Ctl| c.st.bypass.iter().any(|b| !c.st.known.iter().any(|k| k.0.eq_ignore_ascii_case(b) && !k.1.is_empty()));
        if !lost(self) {
            return;
        }
        thread::spawn(|| {
            let games = pick::steam();
            let apps = pick::running();
            later(move |c| c.backfilled(games, apps));
        });
    }

    fn backfilled(&mut self, games: Vec<(String, String, String)>, apps: Vec<(String, String, String)>) {
        let mut changed = false;
        for b in self.st.bypass.clone() {
            if self.st.known.iter().any(|k| k.0.eq_ignore_ascii_case(&b) && !k.1.is_empty()) {
                continue;
            }
            let hit = games.iter().find(|g| g.1.eq_ignore_ascii_case(&b)).map(|g| (g.2.clone(), g.0.clone())).or_else(|| apps.iter().find(|g| g.1.eq_ignore_ascii_case(&b)).map(|g| (g.2.clone(), String::new())));
            if let Some((path, name)) = hit {
                self.st.known.retain(|k| !k.0.eq_ignore_ascii_case(&b));
                self.st.known.push((b, path.replace(['|', '>'], ""), name.replace(['|', '>'], "")));
                changed = true;
            }
        }
        if changed {
            self.st.save();
            self.show_bypass();
        }
    }

    fn show_lists(&self) {
        let v: Vec<Pick> = self
            .st
            .bypass
            .iter()
            .map(|x| {
                let k = self.st.known.iter().find(|k| k.0.eq_ignore_ascii_case(x));
                self.entry(x, k.map_or("", |k| k.1.as_str()), k.map_or("", |k| k.2.as_str()))
            })
            .collect();
        self.ui(|s| s.set_bypass(ModelRc::new(VecModel::from(v))));
        self.show_seen();
        self.show_picks();
    }

    fn entry(&self, exe: &str, path: &str, name: &str) -> Pick {
        let key = path.to_lowercase();
        let title = if !name.is_empty() { name } else { self.titles.get(&key).map_or(exe, String::as_str) };
        Pick {
            label: title.into(),
            exe: exe.into(),
            hint: if title.eq_ignore_ascii_case(exe) { "".into() } else { exe.into() },
            icon: self.icons.get(&key).cloned().unwrap_or_default(),
        }
    }

    fn show_seen(&self) {
        let v: Vec<Pick> = self
            .seen
            .iter()
            .filter(|(n, _)| !self.st.bypass.iter().any(|b| b.eq_ignore_ascii_case(n)))
            .map(|(n, p)| self.entry(n, p, ""))
            .collect();
        self.ui(|s| s.set_seen_apps(ModelRc::new(VecModel::from(v))));
    }

    fn open_picks(&mut self, kind: i32) {
        self.picks.clear();
        self.query.clear();
        self.ui(|s| {
            s.set_pick_kind(kind);
            s.set_pick_busy(true);
        });
        self.show_picks();
        thread::spawn(move || {
            let v = if kind == 1 { pick::steam() } else { pick::running() };
            later(move |c| c.picked(kind, v));
        });
    }

    fn picked(&mut self, kind: i32, v: Vec<(String, String, String)>) {
        if self.ui.upgrade().filter(|u| u.global::<State>().get_pick_kind() == kind).is_none() {
            return;
        }
        let paths: Vec<String> = v.iter().map(|p| p.2.clone()).collect();
        self.picks = v;
        self.ui(|s| s.set_pick_busy(false));
        self.show_picks();
        self.fetch(paths);
    }

    fn fetch(&mut self, paths: Vec<String>) {
        let Some(u) = self.ui.upgrade() else { return };
        if self.icons.len() > 500 {
            self.icons.clear();
            self.titles.clear();
        }
        let mut need: Vec<String> = paths.iter().map(|p| p.to_lowercase()).filter(|p| !self.icons.contains_key(p)).collect();
        need.sort();
        need.dedup();
        if need.is_empty() {
            return;
        }
        for p in &need {
            self.icons.insert(p.clone(), Image::default());
        }
        let size = (20.0 * u.window().scale_factor()).round() as u32;
        thread::spawn(move || {
            let got: Vec<(String, Option<Px>, Option<String>)> = need
                .into_iter()
                .map(|p| {
                    let i = icon::load(&p, size);
                    let t = icon::title(&p);
                    (p, i, t)
                })
                .collect();
            later(move |c| c.iconed(got));
        });
    }

    fn iconed(&mut self, got: Vec<(String, Option<Px>, Option<String>)>) {
        for (p, i, t) in got {
            if let Some(i) = i {
                self.icons.insert(p.clone(), Image::from_rgba8(i));
            }
            if let Some(t) = t {
                self.titles.insert(p, t);
            }
        }
        self.show_lists();
    }

    fn find_picks(&mut self, q: String) {
        self.query = q.trim().to_lowercase();
        self.show_picks();
    }

    fn show_picks(&self) {
        let v: Vec<Pick> = self
            .picks
            .iter()
            .filter(|(_, x, _)| !self.st.bypass.iter().any(|b| b.eq_ignore_ascii_case(x)))
            .map(|(n, x, p)| self.entry(x, p, if n == x { "" } else { n }))
            .filter(|e| e.label.to_lowercase().contains(&self.query) || e.exe.to_lowercase().contains(&self.query))
            .collect();
        self.ui(|s| s.set_picks(ModelRc::new(VecModel::from(v))));
    }

    fn add_app(&mut self, a: &str) {
        let full = a.trim().trim_matches('"');
        let a = full.rsplit(['\\', '/']).next().unwrap_or("");
        let a: String = a.chars().filter(|c| !matches!(c, ',' | '"' | '\'' | '\n' | '\r' | '|' | '>')).collect();
        let a = a.trim();
        if a.is_empty() {
            return;
        }
        let a = if a.contains('.') { a.to_string() } else { format!("{a}.exe") };
        if self.st.bypass.iter().any(|b| b.eq_ignore_ascii_case(&a)) {
            return;
        }
        let picked = self.picks.iter().find(|p| p.1.eq_ignore_ascii_case(&a));
        let path = if full.contains(['\\', '/']) {
            full.to_string()
        } else {
            picked
                .map(|p| p.2.clone())
                .filter(|p| !p.is_empty())
                .or_else(|| self.seen.iter().find(|s| s.0.eq_ignore_ascii_case(&a)).map(|s| s.1.clone()))
                .unwrap_or_default()
        };
        let name = picked.filter(|p| p.0 != p.1).map(|p| p.0.clone()).unwrap_or_default();
        let path: String = path.chars().filter(|c| !matches!(c, '|' | '>')).collect();
        if !path.is_empty() || !name.is_empty() {
            self.st.known.push((a.clone(), path, name.replace(['|', '>'], "")));
        }
        self.st.bypass.push(a);
        self.st.bypass.sort_by_key(|s| s.to_lowercase());
        self.st.save();
        self.show_bypass();
        self.reload();
    }

    fn remove_app(&mut self, a: &str) {
        self.st.bypass.retain(|b| !b.eq_ignore_ascii_case(a));
        self.st.known.retain(|k| !k.0.eq_ignore_ascii_case(a));
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
