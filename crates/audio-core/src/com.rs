//! Per-thread COM initialization.

use std::marker::PhantomData;

use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

use crate::error::{Error, Result};

/// Keeps COM initialized on the current thread for as long as it lives.
///
/// Create one at the top of every thread that touches the audio layer and keep
/// it alive until every COM wrapper created on that thread has been dropped.
#[derive(Debug)]
pub struct ComGuard {
    uninitialize: bool,
    // COM initialization is per thread: the guard must be dropped where it was created.
    _not_send: PhantomData<*const ()>,
}

impl ComGuard {
    /// Joins the multithreaded apartment.
    ///
    /// If the thread already belongs to a single-threaded apartment (for
    /// example the UI thread), COM is usable as is and the guard does nothing.
    pub fn new() -> Result<Self> {
        // SAFETY: plain FFI call; the reserved parameter must be null.
        let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if hr.is_ok() {
            // S_OK and S_FALSE both require a matching CoUninitialize.
            Ok(Self {
                uninitialize: true,
                _not_send: PhantomData,
            })
        } else if hr == RPC_E_CHANGED_MODE {
            Ok(Self {
                uninitialize: false,
                _not_send: PhantomData,
            })
        } else {
            Err(Error::Windows(hr.into()))
        }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.uninitialize {
            // SAFETY: balances the successful CoInitializeEx made by `new` on this thread.
            unsafe { CoUninitialize() };
        }
    }
}
