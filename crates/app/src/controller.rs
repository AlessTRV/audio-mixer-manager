//! Controller thread: owns every COM object, applies commands and publishes
//! [`Snapshot`]s.
//!
//! The UI, the tray and the hotkeys never touch the audio layer directly: they
//! send [`Command`]s through [`Shared`] and read the latest snapshot from it.

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use audio_core::com::ComGuard;
use audio_core::device::{DeviceEnumerator, DeviceId, DeviceWatch};
use audio_core::events::{self, AudioEvent};
use audio_core::policy::{self, AppRouting};
use audio_core::process::{self, IconRgba, ProcessInfo};
use audio_core::session::{self, Session, SessionState, SessionWatch};

use crate::actions::MoveMode;
use crate::model::{AppEntry, Command, Output, SYSTEM_SOUNDS_KEY, Snapshot};

/// Peak meter refresh period while the window is visible.
const METER_PERIOD: Duration = Duration::from_millis(50);

enum Msg {
    Command(Command),
    Audio,
}

/// Handle to the controller, cheap to clone and usable from any thread.
#[derive(Clone)]
pub struct Shared {
    snapshot: Arc<Mutex<Snapshot>>,
    tx: Sender<Msg>,
}

impl Shared {
    /// Copy of the latest snapshot.
    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        self.snapshot
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Queues a command. Ignored once the controller has stopped.
    pub fn send(&self, command: Command) {
        let _ = self.tx.send(Msg::Command(command));
    }
}

/// Starts the controller. `notify` runs after every published snapshot (on the
/// controller thread) and should wake the UI, e.g. `egui::Context::request_repaint`.
pub fn spawn(
    notify: impl Fn() + Send + Sync + 'static,
) -> std::io::Result<(Shared, JoinHandle<()>)> {
    let snapshot = Arc::new(Mutex::new(Snapshot::default()));
    let (tx, rx) = mpsc::channel();
    let shared = Shared {
        snapshot: Arc::clone(&snapshot),
        tx: tx.clone(),
    };
    let handle = thread::Builder::new()
        .name("audio-controller".into())
        .spawn(move || run(&rx, &tx, snapshot, Box::new(notify)))?;
    Ok((shared, handle))
}

fn run(
    rx: &Receiver<Msg>,
    tx: &Sender<Msg>,
    snapshot: Arc<Mutex<Snapshot>>,
    notify: Box<dyn Fn() + Send + Sync>,
) {
    let _com = match ComGuard::new() {
        Ok(guard) => guard,
        Err(err) => {
            tracing::error!("COM initialization failed: {err}");
            return;
        }
    };
    let mut state = match State::new(tx.clone(), snapshot, notify) {
        Ok(state) => state,
        Err(err) => {
            tracing::error!("audio controller failed to start: {err}");
            return;
        }
    };
    state.full_refresh();

    loop {
        let first = if state.metering {
            match rx.recv_timeout(METER_PERIOD) {
                Ok(msg) => Some(msg),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        } else {
            match rx.recv() {
                Ok(msg) => Some(msg),
                Err(_) => break,
            }
        };
        let Some(first) = first else {
            state.update_peaks();
            continue;
        };

        // Handle everything queued in one go so bursts of events cost one refresh.
        let mut work = Work::default();
        for msg in std::iter::once(first).chain(rx.try_iter()) {
            match msg {
                Msg::Audio => state.collect_audio_events(&mut work),
                Msg::Command(Command::Shutdown) => return,
                Msg::Command(command) => state.apply(&command, &mut work),
            }
        }
        if work.rewatch {
            state.rewatch();
        }
        if work.refresh {
            state.refresh();
        }
    }
}

#[derive(Default)]
struct Work {
    /// Device or session set changed: re-register notifications.
    rewatch: bool,
    /// Anything changed: rebuild and publish the snapshot.
    refresh: bool,
}

struct State {
    enumerator: DeviceEnumerator,
    routing: Option<AppRouting>,
    events: Receiver<AudioEvent>,
    sender: events::EventSender,
    device_watch: Option<DeviceWatch>,
    session_watch: Option<SessionWatch>,
    sessions: Vec<Session>,
    processes: HashMap<u32, ProcessInfo>,
    icons: HashMap<String, Option<Arc<IconRgba>>>,
    usage: Usage,
    snapshot: Arc<Mutex<Snapshot>>,
    notify: Box<dyn Fn() + Send + Sync>,
    metering: bool,
    error: Option<String>,
    generation: u64,
}

impl State {
    fn new(
        tx: Sender<Msg>,
        snapshot: Arc<Mutex<Snapshot>>,
        notify: Box<dyn Fn() + Send + Sync>,
    ) -> audio_core::Result<Self> {
        let enumerator = DeviceEnumerator::new()?;
        let routing = match AppRouting::new() {
            Ok(routing) => Some(routing),
            Err(err) => {
                tracing::warn!("per-app routing unavailable: {err}");
                None
            }
        };
        let (sender, events) = events::channel();
        let sender = sender.with_waker(move || {
            let _ = tx.send(Msg::Audio);
        });
        Ok(Self {
            enumerator,
            routing,
            events,
            sender,
            device_watch: None,
            session_watch: None,
            sessions: Vec::new(),
            processes: HashMap::new(),
            icons: HashMap::new(),
            usage: Usage::default(),
            snapshot,
            notify,
            metering: false,
            error: None,
            generation: 0,
        })
    }

    fn full_refresh(&mut self) {
        match self.enumerator.watch(self.sender.clone()) {
            Ok(watch) => self.device_watch = Some(watch),
            Err(err) => tracing::warn!("device notifications unavailable: {err}"),
        }
        self.rewatch();
        self.refresh();
    }

    fn collect_audio_events(&mut self, work: &mut Work) {
        for event in self.events.try_iter() {
            match event {
                AudioEvent::DevicesChanged | AudioEvent::SessionsChanged => {
                    work.rewatch = true;
                    work.refresh = true;
                }
                AudioEvent::DefaultOutputChanged | AudioEvent::SessionVolumeChanged => {
                    work.refresh = true;
                }
            }
        }
    }

    /// Re-registers session notifications so new sessions are tracked too.
    fn rewatch(&mut self) {
        // Drop the old registrations before creating new ones.
        self.session_watch = None;
        match SessionWatch::new(&self.enumerator, &self.sender) {
            Ok(watch) => self.session_watch = Some(watch),
            Err(err) => tracing::warn!("session notifications unavailable: {err}"),
        }
    }

    fn apply(&mut self, command: &Command, work: &mut Work) {
        let result = match command {
            Command::SetMuted { app, muted } => self.for_app_sessions(app, |s| s.set_muted(*muted)),
            Command::ToggleMuted { app } => {
                let muted = self.app_muted(app);
                self.for_app_sessions(app, |s| s.set_muted(!muted))
            }
            Command::SetVolume { app, volume } => {
                self.for_app_sessions(app, |s| s.set_volume(*volume))
            }
            Command::SetAppOutput { app, output } => self.set_app_output(app, output.as_ref()),
            Command::SetDefaultOutput(id) => policy::set_default_output(id).map_err(Into::into),
            Command::MoveAll { output, mode } => self.move_all(output, *mode),
            Command::SetMetering(on) => {
                self.metering = *on;
                Ok(())
            }
            Command::Refresh => Ok(()),
            Command::Shutdown => unreachable!("handled by the loop"),
        };
        match result {
            Ok(()) => self.error = None,
            Err(err) => {
                tracing::warn!("{command:?} failed: {err:#}");
                self.error = Some(err.to_string());
            }
        }
        work.refresh = true;
        if matches!(command, Command::Refresh) {
            work.rewatch = true;
        }
    }

    fn move_all(&self, output: &DeviceId, mode: MoveMode) -> anyhow::Result<()> {
        // Fail early with a clear message instead of a COM error.
        if !self
            .enumerator
            .active_outputs()?
            .iter()
            .any(|device| &device.id == output)
        {
            anyhow::bail!("output {output} is not connected");
        }
        match mode {
            MoveMode::Reset => {
                policy::set_default_output(output)?;
                if let Some(routing) = &self.routing {
                    routing.clear_all()?;
                }
            }
            MoveMode::KeepRules => policy::set_default_output(output)?,
            MoveMode::ActiveOnly => {
                let Some(routing) = &self.routing else {
                    anyhow::bail!("per-app routing is not available on this Windows build");
                };
                let mut pids: Vec<u32> = self
                    .sessions
                    .iter()
                    .filter(|s| {
                        !s.is_system_sounds() && matches!(s.state(), Ok(SessionState::Active))
                    })
                    .map(Session::pid)
                    .collect();
                pids.sort_unstable();
                pids.dedup();
                for pid in pids {
                    routing.set_output(pid, Some(output))?;
                }
            }
        }
        Ok(())
    }

    fn app_key(&self, session: &Session) -> String {
        if session.is_system_sounds() {
            return SYSTEM_SOUNDS_KEY.to_owned();
        }
        self.processes.get(&session.pid()).map_or_else(
            || format!("pid {}", session.pid()),
            |info| info.exe_name.clone(),
        )
    }

    fn app_muted(&self, app: &str) -> bool {
        let mut any = false;
        for session in self.sessions.iter().filter(|s| self.app_key(s) == app) {
            any = true;
            if !session.muted().unwrap_or(false) {
                return false;
            }
        }
        any
    }

    fn for_app_sessions(
        &self,
        app: &str,
        mut action: impl FnMut(&Session) -> audio_core::Result<()>,
    ) -> anyhow::Result<()> {
        let mut first_error = None;
        for session in self.sessions.iter().filter(|s| self.app_key(s) == app) {
            if let Err(err) = action(session) {
                first_error.get_or_insert(err);
            }
        }
        first_error.map_or(Ok(()), |err| Err(err.into()))
    }

    fn set_app_output(&self, app: &str, output: Option<&DeviceId>) -> anyhow::Result<()> {
        let Some(routing) = &self.routing else {
            anyhow::bail!("per-app routing is not available on this Windows build");
        };
        let mut pids: Vec<u32> = self
            .sessions
            .iter()
            .filter(|s| !s.is_system_sounds() && self.app_key(s) == app)
            .map(Session::pid)
            .collect();
        pids.sort_unstable();
        pids.dedup();
        for pid in pids {
            routing.set_output(pid, output)?;
        }
        Ok(())
    }

    /// Re-reads devices and sessions and publishes a new snapshot.
    fn refresh(&mut self) {
        let outputs = match self.enumerator.active_outputs() {
            Ok(devices) => devices,
            Err(err) => {
                tracing::warn!("cannot list outputs: {err}");
                Vec::new()
            }
        };
        self.sessions = session::sessions(&self.enumerator).unwrap_or_else(|err| {
            tracing::warn!("cannot list sessions: {err}");
            Vec::new()
        });

        // Keep process info only for live pids; look up the new ones once.
        let live: Vec<u32> = self.sessions.iter().map(Session::pid).collect();
        self.processes.retain(|pid, _| live.contains(pid));
        for pid in live {
            self.processes
                .entry(pid)
                .or_insert_with(|| process::process_info(pid));
        }

        let default_output = outputs.iter().find(|d| d.is_default).map(|d| d.id.clone());
        let apps = self.build_apps();
        self.generation += 1;
        let snapshot = Snapshot {
            outputs: outputs
                .into_iter()
                .map(|d| Output {
                    id: d.id,
                    name: d.name,
                })
                .collect(),
            default_output,
            apps,
            routing_available: self.routing.is_some(),
            error: self.error.clone(),
            generation: self.generation,
        };
        self.publish(snapshot);
    }

    fn build_apps(&mut self) -> Vec<AppEntry> {
        let mut apps: Vec<AppEntry> = Vec::new();
        for session in &self.sessions {
            let key = self.app_key(session);
            let state = session.state().unwrap_or(SessionState::Inactive);
            let muted = session.muted().unwrap_or(false);
            let volume = session.volume().unwrap_or(1.0);
            let peak = if self.metering {
                session.peak().unwrap_or(0.0)
            } else {
                0.0
            };

            if let Some(app) = apps.iter_mut().find(|a| a.key == key) {
                if !app.pids.contains(&session.pid()) {
                    app.pids.push(session.pid());
                }
                if !app.devices.contains(session.device_id()) {
                    app.devices.push(session.device_id().clone());
                }
                app.muted &= muted;
                app.volume = app.volume.max(volume);
                app.peak = app.peak.max(peak);
                app.active |= state == SessionState::Active;
                continue;
            }

            let info = self.processes.get(&session.pid());
            let is_system = session.is_system_sounds();
            let display_name = info.map_or_else(|| key.clone(), |i| i.display_name.clone());
            let icon = match info.and_then(|i| i.exe_path.clone()) {
                Some(path) if !is_system => self
                    .icons
                    .entry(key.clone())
                    .or_insert_with(|| process::exe_icon(&path).map(Arc::new))
                    .clone(),
                _ => None,
            };
            let routed_to = match &self.routing {
                Some(routing) if !is_system => routing.output(session.pid()).unwrap_or(None),
                _ => None,
            };
            apps.push(AppEntry {
                key,
                display_name,
                icon,
                pids: vec![session.pid()],
                muted,
                volume,
                peak,
                active: state == SessionState::Active,
                devices: vec![session.device_id().clone()],
                routed_to,
                is_system,
            });
        }
        self.usage.update(&apps, Instant::now());
        self.usage.sort(&mut apps);
        apps
    }

    /// Updates only the peak meters of the current snapshot.
    fn update_peaks(&mut self) {
        let mut peaks: HashMap<String, f32> = HashMap::new();
        for session in &self.sessions {
            let peak = session.peak().unwrap_or(0.0);
            let entry = peaks.entry(self.app_key(session)).or_insert(0.0);
            *entry = entry.max(peak);
        }
        {
            let mut snapshot = self.snapshot.lock().unwrap_or_else(PoisonError::into_inner);
            for app in &mut snapshot.apps {
                app.peak = peaks.get(&app.key).copied().unwrap_or(0.0);
            }
        }
        (self.notify)();
    }

    fn publish(&self, snapshot: Snapshot) {
        *self.snapshot.lock().unwrap_or_else(PoisonError::into_inner) = snapshot;
        (self.notify)();
    }
}

/// Remembers when each application last played, to order the list by use.
#[derive(Debug, Default)]
struct Usage {
    /// When an app currently playing started playing.
    started: HashMap<String, Instant>,
    /// Last time an app was seen playing.
    last_active: HashMap<String, Instant>,
}

impl Usage {
    fn update(&mut self, apps: &[AppEntry], now: Instant) {
        for app in apps {
            if app.active {
                self.started.entry(app.key.clone()).or_insert(now);
                self.last_active.insert(app.key.clone(), now);
            } else {
                self.started.remove(&app.key);
            }
        }
    }

    /// Playing apps first, most recently started on top; then the others,
    /// most recently used on top, never-used ones by name; system sounds last.
    fn sort(&self, apps: &mut [AppEntry]) {
        apps.sort_by(|a, b| {
            a.is_system
                .cmp(&b.is_system)
                .then(b.active.cmp(&a.active))
                .then_with(|| {
                    let recency = if a.active {
                        &self.started
                    } else {
                        &self.last_active
                    };
                    // `None < Some`, so reversing puts the most recent first and unknown last.
                    recency.get(&b.key).cmp(&recency.get(&a.key))
                })
                .then_with(|| {
                    a.display_name
                        .to_lowercase()
                        .cmp(&b.display_name.to_lowercase())
                })
        });
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn app(key: &str, active: bool) -> AppEntry {
        AppEntry {
            key: key.to_owned(),
            display_name: key.to_owned(),
            icon: None,
            pids: Vec::new(),
            muted: false,
            volume: 1.0,
            peak: 0.0,
            active,
            devices: Vec::new(),
            routed_to: None,
            is_system: false,
        }
    }

    fn keys(apps: &[AppEntry]) -> Vec<&str> {
        apps.iter().map(|a| a.key.as_str()).collect()
    }

    #[test]
    fn orders_by_recent_use() {
        let mut usage = Usage::default();
        let t0 = Instant::now();
        // a plays, then b starts later; c never plays.
        usage.update(&[app("a", true), app("b", false), app("c", false)], t0);
        usage.update(
            &[app("a", true), app("b", true), app("c", false)],
            t0 + Duration::from_secs(1),
        );
        let mut apps = vec![app("c", false), app("a", true), app("b", true)];
        usage.sort(&mut apps);
        assert_eq!(keys(&apps), ["b", "a", "c"]);

        // b stops first, a stops later: a is the most recently used.
        usage.update(
            &[app("a", true), app("b", false), app("c", false)],
            t0 + Duration::from_secs(2),
        );
        usage.update(
            &[app("a", false), app("b", false), app("c", false)],
            t0 + Duration::from_secs(3),
        );
        let mut apps = vec![app("c", false), app("b", false), app("a", false)];
        usage.sort(&mut apps);
        assert_eq!(keys(&apps), ["a", "b", "c"]);
    }
}
