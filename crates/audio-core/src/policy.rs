//! Undocumented Windows interfaces for the system default endpoint and for
//! per-application output routing.
//!
//! Nothing here is part of the public Windows SDK. Interface ids, class ids and
//! method order are copied from `EarTrumpet` (MIT, <https://github.com/File-New-Project/EarTrumpet>):
//!
//! - `EarTrumpet/Interop/MMDeviceAPI/IPolicyConfig.cs`
//! - `EarTrumpet/Interop/MMDeviceAPI/PolicyConfigClient.cs`
//! - `EarTrumpet/Interop/MMDeviceAPI/IAudioPolicyConfigFactoryVariantFor21H2.cs`
//! - `EarTrumpet/Interop/MMDeviceAPI/IAudioPolicyConfigFactoryVariantForDownlevel.cs`
//! - `EarTrumpet/DataModel/WindowsAudio/Internal/AudioPolicyConfigService.cs`
//!
//! Unused methods are kept as placeholders: removing one shifts every slot
//! after it and the next call jumps to the wrong function.

// `transmute_ptr_to_ptr` is raised by code that `windows::core::interface` generates.
#![allow(
    non_snake_case,
    clippy::missing_safety_doc,
    clippy::transmute_ptr_to_ptr
)]

use std::ffi::c_void;
use std::ptr;

use windows::Win32::Media::Audio::{
    EDataFlow, ERole, eCommunications, eConsole, eMultimedia, eRender,
};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};
use windows::Win32::System::WinRT::RoGetActivationFactory;
use windows::core::{GUID, HRESULT, HSTRING, IUnknown, IUnknown_Vtbl, PCWSTR, interface};

use crate::device::DeviceId;
use crate::error::{Error, Result};

/// `PolicyConfigClient` coclass. Source: `PolicyConfigClient.cs`.
const CLSID_POLICY_CONFIG_CLIENT: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

/// `IPolicyConfig` as exposed from Windows 7 to 8.1 and from Windows 10 1607
/// onwards. Source: `IPolicyConfig.cs` (`IPolicyConfigWin7`).
#[interface("f8679f50-850a-41cf-9c72-430f290290c8")]
unsafe trait IPolicyConfig: IUnknown {
    fn Unused1(&self) -> HRESULT;
    fn Unused2(&self) -> HRESULT;
    fn Unused3(&self) -> HRESULT;
    fn Unused4(&self) -> HRESULT;
    fn Unused5(&self) -> HRESULT;
    fn Unused6(&self) -> HRESULT;
    fn Unused7(&self) -> HRESULT;
    fn Unused8(&self) -> HRESULT;
    fn GetPropertyValue(
        &self,
        device_id: PCWSTR,
        key: *const c_void,
        value: *mut c_void,
    ) -> HRESULT;
    fn SetPropertyValue(
        &self,
        device_id: PCWSTR,
        key: *const c_void,
        value: *const c_void,
    ) -> HRESULT;
    fn SetDefaultEndpoint(&self, device_id: PCWSTR, role: ERole) -> HRESULT;
    fn SetEndpointVisibility(&self, device_id: PCWSTR, visible: i16) -> HRESULT;
}

/// Declares one variant of the `AudioPolicyConfig` activation factory.
///
/// The interface derives from `IInspectable`; it is declared on top of
/// `IUnknown` with the three `IInspectable` methods spelled out as placeholders,
/// which yields the same vtable. The variants differ only in their interface id.
macro_rules! audio_policy_config_factory {
    ($(#[$meta:meta])* $name:ident, $iid:literal) => {
        $(#[$meta])*
        #[interface($iid)]
        unsafe trait $name: IUnknown {
            // IInspectable
            fn GetIids(&self) -> HRESULT;
            fn GetRuntimeClassName(&self) -> HRESULT;
            fn GetTrustLevel(&self) -> HRESULT;
            // IAudioPolicyConfigFactory
            fn add_CtxVolumeChange(&self) -> HRESULT;
            fn remove_CtxVolumeChanged(&self) -> HRESULT;
            fn add_RingerVibrateStateChanged(&self) -> HRESULT;
            fn remove_RingerVibrateStateChange(&self) -> HRESULT;
            fn SetVolumeGroupGainForId(&self) -> HRESULT;
            fn GetVolumeGroupGainForId(&self) -> HRESULT;
            fn GetActiveVolumeGroupForEndpointId(&self) -> HRESULT;
            fn GetVolumeGroupsForEndpoint(&self) -> HRESULT;
            fn GetCurrentVolumeContext(&self) -> HRESULT;
            fn SetVolumeGroupMuteForId(&self) -> HRESULT;
            fn GetVolumeGroupMuteForId(&self) -> HRESULT;
            fn SetRingerVibrateState(&self) -> HRESULT;
            fn GetRingerVibrateState(&self) -> HRESULT;
            fn SetPreferredChatApplication(&self) -> HRESULT;
            fn ResetPreferredChatApplication(&self) -> HRESULT;
            fn GetPreferredChatApplication(&self) -> HRESULT;
            fn GetCurrentChatApplications(&self) -> HRESULT;
            fn add_ChatContextChanged(&self) -> HRESULT;
            fn remove_ChatContextChanged(&self) -> HRESULT;
            fn SetPersistedDefaultAudioEndpoint(
                &self,
                process_id: u32,
                flow: EDataFlow,
                role: ERole,
                device_id: *mut c_void,
            ) -> HRESULT;
            fn GetPersistedDefaultAudioEndpoint(
                &self,
                process_id: u32,
                flow: EDataFlow,
                role: ERole,
                device_id: *mut *mut c_void,
            ) -> HRESULT;
            fn ClearAllPersistedApplicationDefaultEndpoints(&self) -> HRESULT;
        }
    };
}

audio_policy_config_factory!(
    /// Windows 10 21H2 and later, Windows 11.
    /// Source: `IAudioPolicyConfigFactoryVariantFor21H2.cs`.
    IAudioPolicyConfigFactory21H2,
    "ab3d4648-e242-459f-b02f-541c70306324"
);

audio_policy_config_factory!(
    /// Windows 10 1803 to 21H1.
    /// Source: `IAudioPolicyConfigFactoryVariantForDownlevel.cs`.
    IAudioPolicyConfigFactoryDownlevel,
    "2a59116d-6c4f-45e0-a74f-707e3fef9258"
);

/// Runtime class that hands out the factory. Source: `EarTrumpet` interop helpers.
const AUDIO_POLICY_CONFIG_CLASS: &str = "Windows.Media.Internal.AudioPolicyConfig";

// Device id decoration expected by the factory. Source: `AudioPolicyConfigService.cs`.
const MMDEVAPI_TOKEN: &str = r"\\?\SWD#MMDEVAPI#";
const DEVINTERFACE_AUDIO_RENDER: &str = "#{e6327cad-dcec-4949-ae8a-991e976a79d2}";

/// Makes `id` the system default output for every role (console, multimedia,
/// communications).
pub fn set_default_output(id: &DeviceId) -> Result<()> {
    // SAFETY: standard COM activation; the class id comes from the cited source.
    let policy: IPolicyConfig =
        unsafe { CoCreateInstance(&CLSID_POLICY_CONFIG_CLIENT, None, CLSCTX_ALL) }?;
    let wide: Vec<u16> = id
        .as_str()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    for role in [eConsole, eMultimedia, eCommunications] {
        // SAFETY: `wide` is a nul-terminated UTF-16 string that outlives the call;
        // the slot and signature match `IPolicyConfigWin7::SetDefaultEndpoint`.
        unsafe { policy.SetDefaultEndpoint(PCWSTR(wide.as_ptr()), role) }.ok()?;
    }
    Ok(())
}

enum Factory {
    Current(IAudioPolicyConfigFactory21H2),
    Downlevel(IAudioPolicyConfigFactoryDownlevel),
}

/// Per-application output routing, the mechanism behind "App volume and device
/// preferences" in Windows Settings.
///
/// Assignments are stored by Windows per application and survive restarts of
/// the application and of this program.
pub struct AppRouting {
    factory: Factory,
}

impl AppRouting {
    /// Connects to the routing service.
    ///
    /// Returns [`Error::Unsupported`] when no known variant of the interface is
    /// available (Windows older than 10 1803, or a future build that changed it).
    pub fn new() -> Result<Self> {
        let class = HSTRING::from(AUDIO_POLICY_CONFIG_CLASS);
        // SAFETY: plain activation call; a failure only means the variant is absent.
        if let Ok(factory) =
            unsafe { RoGetActivationFactory::<IAudioPolicyConfigFactory21H2>(&class) }
        {
            return Ok(Self {
                factory: Factory::Current(factory),
            });
        }
        // SAFETY: as above.
        if let Ok(factory) =
            unsafe { RoGetActivationFactory::<IAudioPolicyConfigFactoryDownlevel>(&class) }
        {
            return Ok(Self {
                factory: Factory::Downlevel(factory),
            });
        }
        Err(Error::Unsupported(
            "per-application audio routing (AudioPolicyConfig)",
        ))
    }

    /// Routes the output of process `pid` to `device`, or back to the system
    /// default when `device` is `None`.
    pub fn set_output(&self, pid: u32, device: Option<&DeviceId>) -> Result<()> {
        // An empty HSTRING is a null handle, which the service reads as "no override".
        let decorated = device.map_or_else(HSTRING::new, |id| HSTRING::from(decorate(id)));
        // SAFETY: HSTRING is a transparent wrapper around the handle; the copy is
        // only borrowed for the duration of the calls and `decorated` outlives them.
        let handle: *mut c_void = unsafe { std::mem::transmute_copy(&decorated) };
        for role in [eMultimedia, eConsole] {
            // SAFETY: slot and signature match the cited interface definition.
            let hr = unsafe {
                match &self.factory {
                    Factory::Current(f) => {
                        f.SetPersistedDefaultAudioEndpoint(pid, eRender, role, handle)
                    }
                    Factory::Downlevel(f) => {
                        f.SetPersistedDefaultAudioEndpoint(pid, eRender, role, handle)
                    }
                }
            };
            hr.ok()?;
        }
        Ok(())
    }

    /// Returns the device process `pid` is routed to, or `None` when it follows
    /// the system default.
    pub fn output(&self, pid: u32) -> Result<Option<DeviceId>> {
        let mut raw: *mut c_void = ptr::null_mut();
        // SAFETY: slot and signature match the cited interface definition; `raw`
        // is a valid out pointer.
        let hr = unsafe {
            match &self.factory {
                Factory::Current(f) => {
                    f.GetPersistedDefaultAudioEndpoint(pid, eRender, eMultimedia, &raw mut raw)
                }
                Factory::Downlevel(f) => {
                    f.GetPersistedDefaultAudioEndpoint(pid, eRender, eMultimedia, &raw mut raw)
                }
            }
        };
        // SAFETY: on success `raw` is an HSTRING handle (possibly null) owned by the
        // caller; on failure it is still null. Wrapping it frees it on drop.
        let decorated: HSTRING = unsafe { std::mem::transmute(raw) };
        hr.ok()?;
        let decorated = decorated.to_string_lossy();
        if decorated.is_empty() {
            Ok(None)
        } else {
            Ok(Some(DeviceId(undecorate(&decorated))))
        }
    }

    /// Removes every per-application assignment, for all applications.
    pub fn clear_all(&self) -> Result<()> {
        // SAFETY: slot and signature match the cited interface definition.
        let hr = unsafe {
            match &self.factory {
                Factory::Current(f) => f.ClearAllPersistedApplicationDefaultEndpoints(),
                Factory::Downlevel(f) => f.ClearAllPersistedApplicationDefaultEndpoints(),
            }
        };
        hr.ok()?;
        Ok(())
    }
}

impl std::fmt::Debug for AppRouting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let variant = match self.factory {
            Factory::Current(_) => "21H2+",
            Factory::Downlevel(_) => "downlevel",
        };
        f.debug_struct("AppRouting")
            .field("variant", &variant)
            .finish()
    }
}

/// Turns an `MMDevice` endpoint id into the device interface path the factory expects.
fn decorate(id: &DeviceId) -> String {
    format!("{MMDEVAPI_TOKEN}{}{DEVINTERFACE_AUDIO_RENDER}", id.as_str())
}

/// Inverse of [`decorate`]; ids that are not decorated are returned unchanged.
fn undecorate(decorated: &str) -> String {
    let id = decorated.strip_prefix(MMDEVAPI_TOKEN).unwrap_or(decorated);
    id.strip_suffix(DEVINTERFACE_AUDIO_RENDER)
        .unwrap_or(id)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "{0.0.0.00000000}.{6b1f4a5e-6f0c-4d7a-9a3e-2f1c0d9e8b7a}";

    #[test]
    fn decorate_wraps_endpoint_id() {
        assert_eq!(
            decorate(&DeviceId(ID.to_owned())),
            r"\\?\SWD#MMDEVAPI#{0.0.0.00000000}.{6b1f4a5e-6f0c-4d7a-9a3e-2f1c0d9e8b7a}#{e6327cad-dcec-4949-ae8a-991e976a79d2}"
        );
    }

    #[test]
    fn undecorate_round_trips() {
        let decorated = decorate(&DeviceId(ID.to_owned()));
        assert_eq!(undecorate(&decorated), ID);
    }

    #[test]
    fn undecorate_leaves_plain_ids_alone() {
        assert_eq!(undecorate(ID), ID);
    }
}
