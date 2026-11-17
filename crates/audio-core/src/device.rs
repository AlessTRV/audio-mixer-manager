//! Audio devices: active render endpoints, the default device and
//! device change notifications.

// Raised by code that `windows::core::implement` generates, not by this module.
#![allow(clippy::inline_always, clippy::ref_as_ptr)]

use std::fmt;

use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Foundation::{ERROR_NOT_FOUND, PROPERTYKEY};
use windows::Win32::Media::Audio::{
    DEVICE_STATE, DEVICE_STATE_ACTIVE, EDataFlow, ERole, IMMDevice, IMMDeviceEnumerator,
    IMMNotificationClient, IMMNotificationClient_Impl, MMDeviceEnumerator, eMultimedia, eRender,
};
use windows::Win32::System::Com::StructuredStorage::{PropVariantClear, PropVariantToStringAlloc};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance, CoTaskMemFree, STGM_READ};
use windows::core::{HRESULT, HSTRING, PCWSTR, PWSTR, implement};

use crate::error::{Error, Result};
use crate::events::{AudioEvent, EventSender};

/// `HRESULT_FROM_WIN32(ERROR_NOT_FOUND)` (`0x80070490`): returned by
/// `GetDefaultAudioEndpoint` when there is no device and by `GetDevice` for an
/// unknown id.
const E_NOTFOUND: HRESULT = HRESULT::from_win32(ERROR_NOT_FOUND.0);

/// Endpoint id of an `MMDevice`, e.g. `{0.0.0.00000000}.{guid}`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceId(pub String);

impl DeviceId {
    /// The raw endpoint id string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// An active output (render) device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub id: DeviceId,
    /// Friendly name shown by Windows; falls back to the id when unreadable.
    pub name: String,
    /// Whether this is the default multimedia output.
    pub is_default: bool,
}

/// Wrapper around `IMMDeviceEnumerator`.
///
/// COM must already be initialized on the thread that creates and uses it.
pub struct DeviceEnumerator {
    inner: IMMDeviceEnumerator,
}

impl DeviceEnumerator {
    /// Creates the system device enumerator. Requires COM on the current thread.
    pub fn new() -> Result<Self> {
        // SAFETY: plain COM activation; the caller has initialized COM on this
        // thread (documented requirement) and no outer object is passed.
        let inner: IMMDeviceEnumerator =
            unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }?;
        Ok(Self { inner })
    }

    /// All active render devices, with friendly name and default flag
    /// (relative to the multimedia role).
    pub fn active_outputs(&self) -> Result<Vec<Device>> {
        let default = self.default_output()?;
        let devices = self
            .active_output_devices()?
            .into_iter()
            .map(|(id, device)| {
                let name = friendly_name(&device).unwrap_or_else(|| id.0.clone());
                let is_default = default.as_ref() == Some(&id);
                Device {
                    id,
                    name,
                    is_default,
                }
            })
            .collect();
        Ok(devices)
    }

    /// Id of the default multimedia output, or `None` when the system has no
    /// output device at all.
    pub fn default_output(&self) -> Result<Option<DeviceId>> {
        // SAFETY: `inner` is a valid enumerator; the arguments are plain enums.
        match unsafe { self.inner.GetDefaultAudioEndpoint(eRender, eMultimedia) } {
            Ok(device) => Ok(Some(device_id(&device)?)),
            Err(err) if err.code() == E_NOTFOUND => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    /// Registers for device notifications. Events are delivered on `sender`
    /// until the returned [`DeviceWatch`] is dropped.
    pub fn watch(&self, sender: EventSender) -> Result<DeviceWatch> {
        let client: IMMNotificationClient = NotificationClient { sender }.into();
        // SAFETY: `client` is a valid COM object implementing the interface;
        // the enumerator keeps its own reference until unregistration.
        unsafe { self.inner.RegisterEndpointNotificationCallback(&client) }?;
        Ok(DeviceWatch {
            enumerator: self.inner.clone(),
            client,
        })
    }

    /// Looks up a device by endpoint id (any state).
    // Not called yet: revisit once the crate can be compiled (M1 verification).
    #[allow(dead_code)]
    pub(crate) fn device(&self, id: &DeviceId) -> Result<IMMDevice> {
        let wide = HSTRING::from(id.as_str());
        // SAFETY: `wide` is a NUL-terminated wide string that outlives the call.
        match unsafe { self.inner.GetDevice(&wide) } {
            Ok(device) => Ok(device),
            Err(err) if err.code() == E_NOTFOUND => Err(Error::DeviceNotFound(id.0.clone())),
            Err(err) => Err(err.into()),
        }
    }

    /// All active render devices with their ids.
    pub(crate) fn active_output_devices(&self) -> Result<Vec<(DeviceId, IMMDevice)>> {
        // SAFETY: `inner` is a valid enumerator; the arguments are plain values.
        let collection = unsafe { self.inner.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) }?;
        // SAFETY: `collection` is a valid device collection.
        let count = unsafe { collection.GetCount() }?;
        let mut out = Vec::with_capacity(count as usize);
        for index in 0..count {
            // SAFETY: `index` is below the count reported by the collection.
            let device = unsafe { collection.Item(index) }?;
            out.push((device_id(&device)?, device));
        }
        Ok(out)
    }
}

/// Keeps a device notification callback registered; unregisters on drop.
pub struct DeviceWatch {
    enumerator: IMMDeviceEnumerator,
    client: IMMNotificationClient,
}

impl Drop for DeviceWatch {
    fn drop(&mut self) {
        // SAFETY: `client` was registered on `enumerator` in `watch`; both are
        // still alive. A failure here cannot be handled meaningfully.
        let _ = unsafe {
            self.enumerator
                .UnregisterEndpointNotificationCallback(&self.client)
        };
    }
}

/// COM object receiving endpoint notifications on system threads. It only
/// forwards a coarse event and returns immediately.
#[implement(IMMNotificationClient)]
struct NotificationClient {
    sender: EventSender,
}

impl IMMNotificationClient_Impl for NotificationClient_Impl {
    fn OnDeviceStateChanged(
        &self,
        _device_id: &PCWSTR,
        _new_state: DEVICE_STATE,
    ) -> windows::core::Result<()> {
        self.sender.send(AudioEvent::DevicesChanged);
        Ok(())
    }

    fn OnDeviceAdded(&self, _device_id: &PCWSTR) -> windows::core::Result<()> {
        self.sender.send(AudioEvent::DevicesChanged);
        Ok(())
    }

    fn OnDeviceRemoved(&self, _device_id: &PCWSTR) -> windows::core::Result<()> {
        self.sender.send(AudioEvent::DevicesChanged);
        Ok(())
    }

    fn OnDefaultDeviceChanged(
        &self,
        flow: EDataFlow,
        _role: ERole,
        _default_device_id: &PCWSTR,
    ) -> windows::core::Result<()> {
        if flow == eRender {
            self.sender.send(AudioEvent::DefaultOutputChanged);
        }
        Ok(())
    }

    fn OnPropertyValueChanged(
        &self,
        _device_id: &PCWSTR,
        key: &PROPERTYKEY,
    ) -> windows::core::Result<()> {
        if *key == PKEY_Device_FriendlyName {
            self.sender.send(AudioEvent::DevicesChanged);
        }
        Ok(())
    }
}

/// Endpoint id of a device.
pub(crate) fn device_id(device: &IMMDevice) -> Result<DeviceId> {
    // SAFETY: `device` is a valid endpoint; the returned string is allocated
    // with CoTaskMemAlloc and owned by us.
    let raw = unsafe { device.GetId() }?;
    // SAFETY: `raw` comes from `GetId` and is not used afterwards.
    Ok(DeviceId(unsafe { take_co_task_string(raw) }))
}

/// Friendly name of a device (`PKEY_Device_FriendlyName`), if readable and not empty.
fn friendly_name(device: &IMMDevice) -> Option<String> {
    // SAFETY: `device` is a valid endpoint.
    let store = unsafe { device.OpenPropertyStore(STGM_READ) }.ok()?;
    // SAFETY: the key is a valid static PROPERTYKEY; we own the returned PROPVARIANT.
    let mut value = unsafe { store.GetValue(&PKEY_Device_FriendlyName) }.ok()?;
    // SAFETY: `value` is an initialized PROPVARIANT.
    let text = unsafe { PropVariantToStringAlloc(&raw const value) };
    // SAFETY: `value` is initialized and not used after being cleared.
    let _ = unsafe { PropVariantClear(&raw mut value) };
    let text = text.ok()?;
    // SAFETY: `text` was allocated by PropVariantToStringAlloc with CoTaskMemAlloc.
    let name = unsafe { take_co_task_string(text) };
    (!name.trim().is_empty()).then_some(name)
}

/// Converts a wide string allocated with `CoTaskMemAlloc` to a `String` and frees it.
///
/// # Safety
/// `text` must be null or a NUL-terminated wide string allocated with
/// `CoTaskMemAlloc`, not freed elsewhere and not used after this call.
unsafe fn take_co_task_string(text: PWSTR) -> String {
    if text.is_null() {
        return String::new();
    }
    // SAFETY: guaranteed by the caller: valid NUL-terminated wide string.
    let value = String::from_utf16_lossy(unsafe { text.as_wide() });
    // SAFETY: guaranteed by the caller: CoTaskMem allocation owned by us.
    unsafe { CoTaskMemFree(Some(text.0.cast_const().cast())) };
    value
}
