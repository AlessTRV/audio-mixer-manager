//! Single instance: a second launch asks the running one to show its window
//! and exits.
//!
//! Both sides use a named auto-reset event. The first process creates it and
//! waits on it from a background thread; a later process finds it already
//! existing, signals it and quits.

use std::thread;

use windows::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, WAIT_OBJECT_0,
};
use windows::Win32::System::Threading::{CreateEventW, INFINITE, SetEvent, WaitForSingleObject};
use windows::core::w;

/// Outcome of [`acquire`].
pub enum Instance {
    /// This is the only instance. Call [`Primary::on_activate`] to be told when
    /// another launch asks for the window.
    Primary(Primary),
    /// Another instance is running and has been asked to show itself.
    Secondary,
}

/// Owner of the activation event.
pub struct Primary {
    // Raw handle value: `HANDLE` is not `Send`, the waiting thread needs it.
    event: usize,
}

/// Claims the single-instance slot, or wakes the instance that holds it.
pub fn acquire() -> windows::core::Result<Instance> {
    // SAFETY: plain FFI call with a static, nul-terminated name and no security attributes.
    let event =
        unsafe { CreateEventW(None, false, false, w!("Local\\AudioMixerManager.Activate")) }?;
    // SAFETY: read immediately after the call that set it.
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        // SAFETY: `event` is a valid handle we own; closed right after use.
        unsafe {
            let signalled = SetEvent(event);
            let _ = CloseHandle(event);
            signalled?;
        }
        return Ok(Instance::Secondary);
    }
    Ok(Instance::Primary(Primary {
        event: event.0 as usize,
    }))
}

impl Primary {
    /// Calls `activate` every time another launch is attempted. The handle is
    /// kept open for the lifetime of the process.
    pub fn on_activate(self, activate: impl Fn() + Send + 'static) -> std::io::Result<()> {
        let event = self.event;
        thread::Builder::new()
            .name("instance-activation".into())
            .spawn(move || {
                let handle = HANDLE(event as *mut core::ffi::c_void);
                loop {
                    // SAFETY: the handle stays open for the whole process lifetime.
                    if unsafe { WaitForSingleObject(handle, INFINITE) } != WAIT_OBJECT_0 {
                        tracing::warn!(
                            "activation event wait failed; single-instance wakeup disabled"
                        );
                        return;
                    }
                    activate();
                }
            })?;
        Ok(())
    }
}
