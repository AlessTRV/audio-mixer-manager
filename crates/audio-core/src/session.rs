//! Per-process audio sessions: pid, state, mute, volume and peak meter.

// Raised by code that `windows::core::implement` generates, not by this module.
#![allow(clippy::inline_always, clippy::ref_as_ptr)]

use std::ptr;

use windows::Win32::Foundation::S_OK;
use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
use windows::Win32::Media::Audio::{
    AudioSessionDisconnectReason, AudioSessionState, AudioSessionStateActive,
    AudioSessionStateExpired, IAudioSessionControl, IAudioSessionControl2, IAudioSessionEvents,
    IAudioSessionEvents_Impl, IAudioSessionManager2, IAudioSessionNotification,
    IAudioSessionNotification_Impl, IMMDevice, ISimpleAudioVolume,
};
use windows::Win32::System::Com::CLSCTX_ALL;
use windows::core::{BOOL, GUID, Interface, PCWSTR, Ref, implement};

use crate::device::{DeviceEnumerator, DeviceId};
use crate::error::Result;
use crate::events::{AudioEvent, EventSender};

/// Lifecycle state of a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionState {
    /// At least one stream of the session is running.
    Active,
    /// The session exists but nothing is playing.
    Inactive,
    /// The owning process closed its streams; the session is going away.
    Expired,
}

impl From<AudioSessionState> for SessionState {
    fn from(state: AudioSessionState) -> Self {
        if state == AudioSessionStateActive {
            Self::Active
        } else if state == AudioSessionStateExpired {
            Self::Expired
        } else {
            Self::Inactive
        }
    }
}

/// One audio session of a process on one output device.
///
/// A process can own several sessions, on the same or on different devices.
pub struct Session {
    pid: u32,
    device_id: DeviceId,
    system_sounds: bool,
    control: IAudioSessionControl2,
    volume: ISimpleAudioVolume,
    meter: IAudioMeterInformation,
}

impl Session {
    fn new(device_id: DeviceId, control: &IAudioSessionControl) -> Result<Self> {
        let control: IAudioSessionControl2 = control.cast()?;
        let volume: ISimpleAudioVolume = control.cast()?;
        let meter: IAudioMeterInformation = control.cast()?;
        // SAFETY: `control` is a valid session control; the call has no preconditions.
        let system_sounds = unsafe { control.IsSystemSoundsSession() } == S_OK;
        // SAFETY: as above. Cross-process sessions report a success code and a pid;
        // a failure is treated like the system sounds session.
        let pid = unsafe { control.GetProcessId() }.unwrap_or(0);
        Ok(Self {
            pid,
            device_id,
            system_sounds,
            control,
            volume,
            meter,
        })
    }

    /// Id of the owning process, `0` for the system sounds session.
    #[must_use]
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Output device the session is rendering to.
    #[must_use]
    pub fn device_id(&self) -> &DeviceId {
        &self.device_id
    }

    /// Whether this is the Windows system sounds session.
    #[must_use]
    pub fn is_system_sounds(&self) -> bool {
        self.system_sounds
    }

    /// Current lifecycle state.
    pub fn state(&self) -> Result<SessionState> {
        // SAFETY: valid interface pointer, no preconditions.
        let state = unsafe { self.control.GetState() }?;
        Ok(state.into())
    }

    /// Whether the session is muted.
    pub fn muted(&self) -> Result<bool> {
        // SAFETY: valid interface pointer, no preconditions.
        let muted = unsafe { self.volume.GetMute() }?;
        Ok(muted.as_bool())
    }

    /// Mutes or unmutes the session.
    pub fn set_muted(&self, muted: bool) -> Result<()> {
        // SAFETY: a null event context is allowed and means "no originator".
        unsafe { self.volume.SetMute(muted, ptr::null()) }?;
        Ok(())
    }

    /// Session volume in `0.0..=1.0`, relative to the device volume.
    pub fn volume(&self) -> Result<f32> {
        // SAFETY: valid interface pointer, no preconditions.
        Ok(unsafe { self.volume.GetMasterVolume() }?)
    }

    /// Sets the session volume; values outside `0.0..=1.0` are clamped.
    pub fn set_volume(&self, volume: f32) -> Result<()> {
        let level = if volume.is_nan() {
            0.0
        } else {
            volume.clamp(0.0, 1.0)
        };
        // SAFETY: the level is within the documented range; null event context is allowed.
        unsafe { self.volume.SetMasterVolume(level, ptr::null()) }?;
        Ok(())
    }

    /// Current peak level in `0.0..=1.0`; above zero means audio is audibly playing.
    pub fn peak(&self) -> Result<f32> {
        // SAFETY: valid interface pointer, no preconditions.
        Ok(unsafe { self.meter.GetPeakValue() }?)
    }
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("pid", &self.pid)
            .field("device_id", &self.device_id)
            .field("system_sounds", &self.system_sounds)
            .finish_non_exhaustive()
    }
}

fn session_manager(device: &IMMDevice) -> Result<IAudioSessionManager2> {
    // SAFETY: valid device; no activation parameters are required for this interface.
    Ok(unsafe { device.Activate(CLSCTX_ALL, None) }?)
}

/// Calls `visit` for every session control of `manager`, skipping the ones that
/// disappear while enumerating.
fn for_each_control(
    manager: &IAudioSessionManager2,
    mut visit: impl FnMut(IAudioSessionControl),
) -> Result<()> {
    // SAFETY: valid interface pointers; indexes stay within the reported count.
    unsafe {
        let enumerator = manager.GetSessionEnumerator()?;
        let count = enumerator.GetCount()?;
        for index in 0..count {
            if let Ok(control) = enumerator.GetSession(index) {
                visit(control);
            }
        }
    }
    Ok(())
}

/// Returns the live sessions of every active output device.
///
/// Expired sessions are left out. A device that disappears while enumerating
/// is skipped rather than failing the whole call.
pub fn sessions(enumerator: &DeviceEnumerator) -> Result<Vec<Session>> {
    let mut found = Vec::new();
    for (device_id, device) in enumerator.active_output_devices()? {
        let Ok(manager) = session_manager(&device) else {
            continue;
        };
        // A device that fails half way still contributes the sessions seen so far.
        let _ = for_each_control(&manager, |control| {
            if let Ok(session) = Session::new(device_id.clone(), &control)
                && !matches!(session.state(), Ok(SessionState::Expired))
            {
                found.push(session);
            }
        });
    }
    Ok(found)
}

#[implement(IAudioSessionNotification)]
struct SessionCreated {
    sender: EventSender,
}

#[allow(non_snake_case)]
impl IAudioSessionNotification_Impl for SessionCreated_Impl {
    fn OnSessionCreated(
        &self,
        _newsession: Ref<IAudioSessionControl>,
    ) -> windows::core::Result<()> {
        self.sender.send(AudioEvent::SessionsChanged);
        Ok(())
    }
}

#[implement(IAudioSessionEvents)]
struct SessionEvents {
    sender: EventSender,
}

#[allow(non_snake_case)]
impl IAudioSessionEvents_Impl for SessionEvents_Impl {
    fn OnDisplayNameChanged(
        &self,
        _newdisplayname: &PCWSTR,
        _eventcontext: *const GUID,
    ) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnIconPathChanged(
        &self,
        _newiconpath: &PCWSTR,
        _eventcontext: *const GUID,
    ) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnSimpleVolumeChanged(
        &self,
        _newvolume: f32,
        _newmute: BOOL,
        _eventcontext: *const GUID,
    ) -> windows::core::Result<()> {
        self.sender.send(AudioEvent::SessionVolumeChanged);
        Ok(())
    }

    fn OnChannelVolumeChanged(
        &self,
        _channelcount: u32,
        _newchannelvolumearray: *const f32,
        _changedchannel: u32,
        _eventcontext: *const GUID,
    ) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnGroupingParamChanged(
        &self,
        _newgroupingparam: *const GUID,
        _eventcontext: *const GUID,
    ) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnStateChanged(&self, _newstate: AudioSessionState) -> windows::core::Result<()> {
        self.sender.send(AudioEvent::SessionsChanged);
        Ok(())
    }

    fn OnSessionDisconnected(
        &self,
        _disconnectreason: AudioSessionDisconnectReason,
    ) -> windows::core::Result<()> {
        self.sender.send(AudioEvent::SessionsChanged);
        Ok(())
    }
}

/// Subscription to session changes on every active output device.
///
/// The watch covers the devices and sessions that exist when it is created.
/// Sessions created later are announced with [`AudioEvent::SessionsChanged`]
/// but their own volume and state changes are not tracked until the watch is
/// rebuilt: create a new one after `SessionsChanged` and `DevicesChanged`.
///
/// Dropping the watch unregisters everything. Drop it on the thread that
/// created it and never from inside a notification callback.
pub struct SessionWatch {
    managers: Vec<(IAudioSessionManager2, IAudioSessionNotification)>,
    sessions: Vec<(IAudioSessionControl, IAudioSessionEvents)>,
}

impl SessionWatch {
    /// Registers the notifications and starts sending events to `sender`.
    pub fn new(enumerator: &DeviceEnumerator, sender: &EventSender) -> Result<Self> {
        let mut watch = Self {
            managers: Vec::new(),
            sessions: Vec::new(),
        };
        for (_, device) in enumerator.active_output_devices()? {
            let Ok(manager) = session_manager(&device) else {
                continue;
            };

            let notification: IAudioSessionNotification = SessionCreated {
                sender: sender.clone(),
            }
            .into();
            // SAFETY: both pointers are valid; the registration is undone in `Drop`.
            if unsafe { manager.RegisterSessionNotification(&notification) }.is_err() {
                continue;
            }

            // Windows only starts delivering session-created notifications after
            // the sessions of the device have been enumerated once.
            let sessions = &mut watch.sessions;
            let enumerated = for_each_control(&manager, |control| {
                let events: IAudioSessionEvents = SessionEvents {
                    sender: sender.clone(),
                }
                .into();
                // SAFETY: both pointers are valid; the registration is undone in `Drop`.
                if unsafe { control.RegisterAudioSessionNotification(&events) }.is_ok() {
                    sessions.push((control, events));
                }
            });
            // Keep the manager registered even if enumeration failed half way.
            let _ = enumerated;
            watch.managers.push((manager, notification));
        }
        Ok(watch)
    }
}

impl Drop for SessionWatch {
    fn drop(&mut self) {
        for (control, events) in self.sessions.drain(..) {
            // SAFETY: unregisters exactly what `new` registered. Failure only means
            // the session is already gone.
            let _ = unsafe { control.UnregisterAudioSessionNotification(&events) };
        }
        for (manager, notification) in self.managers.drain(..) {
            // SAFETY: as above, for the session-created notification.
            let _ = unsafe { manager.UnregisterSessionNotification(&notification) };
        }
    }
}

impl std::fmt::Debug for SessionWatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionWatch")
            .field("devices", &self.managers.len())
            .field("sessions", &self.sessions.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use windows::Win32::Media::Audio::AudioSessionStateInactive;

    use super::*;

    #[test]
    fn state_conversion() {
        assert_eq!(
            SessionState::from(AudioSessionStateActive),
            SessionState::Active
        );
        assert_eq!(
            SessionState::from(AudioSessionStateInactive),
            SessionState::Inactive
        );
        assert_eq!(
            SessionState::from(AudioSessionStateExpired),
            SessionState::Expired
        );
    }
}
