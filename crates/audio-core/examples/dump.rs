//! Manual check of the audio layer.
//!
//! Without arguments it only reads: lists the active outputs and the audio
//! sessions. Subcommands that change the system state:
//!
//! ```text
//! dump mute <pid> on|off
//! dump volume <pid> <0-100>
//! dump route <pid> <device-index>|default
//! dump default <device-index>
//! dump clear-routes
//! dump watch
//! ```

use std::error::Error;

use audio_core::com::ComGuard;
use audio_core::device::{Device, DeviceEnumerator, DeviceId};
use audio_core::events;
use audio_core::policy::{AppRouting, set_default_output};
use audio_core::process::process_info;
use audio_core::session::{Session, SessionState, SessionWatch, sessions};

type AnyResult<T> = Result<T, Box<dyn Error>>;

const USAGE: &str = "usage:
  dump                                 list outputs and sessions (read-only)
  dump mute <pid> on|off               mute or unmute every session of a process
  dump volume <pid> <0-100>            set the volume of every session of a process
  dump route <pid> <device-index>|default
                                       route a process to an output, or back to the default
  dump default <device-index>          set the system default output
  dump clear-routes                    remove every per-app route
  dump watch                           print audio events until Ctrl+C";

fn main() -> AnyResult<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();

    let _com = ComGuard::new()?;
    let enumerator = DeviceEnumerator::new()?;

    match args.as_slice() {
        [] => dump(&enumerator),
        ["mute", pid, state] => {
            let muted = match *state {
                "on" => true,
                "off" => false,
                other => return Err(format!("expected on|off, got {other:?}").into()),
            };
            let targets = sessions_of(&enumerator, parse_pid(pid)?)?;
            for session in &targets {
                session.set_muted(muted)?;
            }
            println!("{} session(s) updated", targets.len());
            Ok(())
        }
        ["volume", pid, value] => {
            let percent: u8 = value.parse()?;
            if percent > 100 {
                return Err("volume must be between 0 and 100".into());
            }
            let targets = sessions_of(&enumerator, parse_pid(pid)?)?;
            for session in &targets {
                session.set_volume(f32::from(percent) / 100.0)?;
            }
            println!("{} session(s) updated", targets.len());
            Ok(())
        }
        ["route", pid, target] => {
            let pid = parse_pid(pid)?;
            let routing = AppRouting::new()?;
            if *target == "default" {
                routing.set_output(pid, None)?;
                println!("pid {pid} follows the system default");
            } else {
                let devices = enumerator.active_outputs()?;
                let device = device_at(&devices, target)?;
                routing.set_output(pid, Some(&device.id))?;
                println!("pid {pid} routed to {}", device.name);
            }
            Ok(())
        }
        ["default", index] => {
            let devices = enumerator.active_outputs()?;
            let device = device_at(&devices, index)?;
            set_default_output(&device.id)?;
            println!("default output set to {}", device.name);
            Ok(())
        }
        ["clear-routes"] => {
            AppRouting::new()?.clear_all()?;
            println!("all per-app routes removed");
            Ok(())
        }
        ["watch"] => watch(&enumerator),
        _ => {
            println!("{USAGE}");
            Err("invalid arguments".into())
        }
    }
}

/// Read-only listing of outputs and sessions.
fn dump(enumerator: &DeviceEnumerator) -> AnyResult<()> {
    let devices = enumerator.active_outputs()?;
    println!("Outputs:");
    for (index, device) in devices.iter().enumerate() {
        let marker = if device.is_default { '*' } else { ' ' };
        println!("  [{index}]{marker} {}  ({})", device.name, device.id);
    }

    let routing = match AppRouting::new() {
        Ok(routing) => Some(routing),
        Err(err) => {
            println!("per-app routing unavailable: {err}");
            None
        }
    };

    println!("Sessions:");
    for session in sessions(enumerator)? {
        let pid = session.pid();
        let info = process_info(pid);
        let state = match session.state() {
            Ok(SessionState::Active) => "active".to_owned(),
            Ok(SessionState::Inactive) => "inactive".to_owned(),
            Ok(SessionState::Expired) => "expired".to_owned(),
            Err(err) => format!("error: {err}"),
        };
        let muted = match session.muted() {
            Ok(true) => "muted".to_owned(),
            Ok(false) => "unmuted".to_owned(),
            Err(err) => format!("mute error: {err}"),
        };
        let volume = match session.volume() {
            Ok(volume) => format!("{:.0}%", volume * 100.0),
            Err(err) => format!("volume error: {err}"),
        };
        let peak = match session.peak() {
            Ok(peak) => format!("{peak:.2}"),
            Err(err) => format!("peak error: {err}"),
        };
        let device = device_label(&devices, session.device_id());
        let route = match (&routing, session.is_system_sounds()) {
            (None, _) => "n/a".to_owned(),
            (Some(_), true) => "-".to_owned(),
            (Some(routing), false) => match routing.output(pid) {
                Ok(None) => "default".to_owned(),
                Ok(Some(id)) => device_label(&devices, &id),
                Err(err) => format!("error: {err}"),
            },
        };
        println!(
            "  pid {pid:>6}  {:<24} {:<28} {state:<8} {muted:<7} vol {volume:>4}  peak {peak}  dev {device}  route {route}",
            info.exe_name, info.display_name,
        );
    }
    Ok(())
}

/// Prints device and session events until the process is interrupted.
fn watch(enumerator: &DeviceEnumerator) -> AnyResult<()> {
    let (sender, receiver) = events::channel();
    let _device_watch = enumerator.watch(sender.clone())?;
    let _session_watch = SessionWatch::new(enumerator, &sender)?;
    println!("watching audio events, press Ctrl+C to stop");
    // The watches keep their senders alive, so this only ends on Ctrl+C.
    for event in receiver {
        println!("{event:?}");
    }
    Ok(())
}

/// All sessions owned by `pid`; an error if there is none.
fn sessions_of(enumerator: &DeviceEnumerator, pid: u32) -> AnyResult<Vec<Session>> {
    let found: Vec<Session> = sessions(enumerator)?
        .into_iter()
        .filter(|session| session.pid() == pid)
        .collect();
    if found.is_empty() {
        return Err(format!("no audio session for pid {pid}").into());
    }
    Ok(found)
}

fn parse_pid(text: &str) -> AnyResult<u32> {
    text.parse()
        .map_err(|err| format!("invalid pid {text:?}: {err}").into())
}

fn device_at<'a>(devices: &'a [Device], index: &str) -> AnyResult<&'a Device> {
    let index: usize = index
        .parse()
        .map_err(|err| format!("invalid device index {index:?}: {err}"))?;
    devices
        .get(index)
        .ok_or_else(|| format!("device index {index} out of range (0..{})", devices.len()).into())
}

/// Index of a device in the listing, or its raw id when it is not an active output.
fn device_label(devices: &[Device], id: &DeviceId) -> String {
    devices
        .iter()
        .position(|device| &device.id == id)
        .map_or_else(|| id.to_string(), |index| format!("[{index}]"))
}
