use crate::Result;
use anyhow::{anyhow, Context};
use std::process::Command;

#[derive(Debug, Clone, Copy)]
pub enum PrivacyPane {
    Accessibility,
    InputMonitoring,
    ScreenRecording,
}

impl PrivacyPane {
    fn url(self) -> &'static str {
        match self {
            PrivacyPane::Accessibility => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
            }
            PrivacyPane::InputMonitoring => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvents"
            }
            PrivacyPane::ScreenRecording => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"
            }
        }
    }
}

/// Open the specified System Settings privacy pane to guide the user manually.
pub fn open_privacy_pane(pane: PrivacyPane) -> Result<()> {
    let status = Command::new("open")
        .arg(pane.url())
        .status()
        .context("failed to open System Settings")?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("open command returned non-zero status: {status}"))
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use crate::Result;
    use anyhow::anyhow;
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFMutableDictionary;
    use core_foundation::string::CFString;
    use core_foundation_sys::dictionary::CFDictionaryRef;
    use core_foundation_sys::string::CFStringRef;

    type IOHIDRequestType = u32;

    const K_IOHID_REQUEST_TYPE_LISTEN_EVENT: IOHIDRequestType = 1;
    const K_IO_RETURN_SUCCESS: i32 = 0;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
        fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
        static kAXTrustedCheckOptionPrompt: CFStringRef;
        fn CGPreflightScreenCaptureAccess() -> bool;
        fn CGRequestScreenCaptureAccess() -> bool;
    }

    #[link(name = "IOKit", kind = "framework")]
    extern "C" {
        fn IOHIDCheckAccess(access_type: IOHIDRequestType) -> bool;
        fn IOHIDRequestAccess(access_type: IOHIDRequestType) -> i32;
    }

    pub fn is_accessibility_permission_granted() -> Result<bool> {
        Ok(unsafe { AXIsProcessTrusted() })
    }

    pub fn prompt_accessibility_permission() -> Result<bool> {
        unsafe {
            let mut options = CFMutableDictionary::new();
            let key = CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt);
            let value = CFBoolean::true_value();
            options.set(key.clone(), value.clone());

            Ok(AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()))
        }
    }

    pub fn is_input_monitoring_permission_granted() -> Result<bool> {
        Ok(unsafe { IOHIDCheckAccess(K_IOHID_REQUEST_TYPE_LISTEN_EVENT) })
    }

    pub fn prompt_input_monitoring_permission() -> Result<bool> {
        let status = unsafe { IOHIDRequestAccess(K_IOHID_REQUEST_TYPE_LISTEN_EVENT) };
        if status == K_IO_RETURN_SUCCESS {
            is_input_monitoring_permission_granted()
        } else {
            Err(anyhow!("IOHIDRequestAccess returned status {status}"))
        }
    }

    pub fn is_screen_recording_permission_granted() -> Result<bool> {
        Ok(unsafe { CGPreflightScreenCaptureAccess() })
    }

    pub fn prompt_screen_recording_permission() -> Result<bool> {
        Ok(unsafe { CGRequestScreenCaptureAccess() })
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use crate::Result;
    use anyhow::anyhow;

    fn env_flag(name: &str) -> bool {
        std::env::var(name)
            .map(|value| value.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    }

    pub fn is_accessibility_permission_granted() -> Result<bool> {
        Ok(env_flag("TILLERS_PERMISSION_ACCESSIBILITY"))
    }

    pub fn prompt_accessibility_permission() -> Result<bool> {
        Ok(env_flag("TILLERS_PERMISSION_ACCESSIBILITY"))
    }

    pub fn is_input_monitoring_permission_granted() -> Result<bool> {
        Ok(env_flag("TILLERS_PERMISSION_INPUT_MONITORING"))
    }

    pub fn prompt_input_monitoring_permission() -> Result<bool> {
        Ok(env_flag("TILLERS_PERMISSION_INPUT_MONITORING"))
    }

    pub fn is_screen_recording_permission_granted() -> Result<bool> {
        Ok(env_flag("TILLERS_PERMISSION_SCREEN_RECORDING"))
    }

    pub fn prompt_screen_recording_permission() -> Result<bool> {
        Ok(env_flag("TILLERS_PERMISSION_SCREEN_RECORDING"))
    }

    pub fn open_privacy_pane(_: super::PrivacyPane) -> Result<()> {
        Err(anyhow!(
            "opening System Settings is not supported on this platform"
        ))
    }
}

pub use platform::{
    is_accessibility_permission_granted, is_input_monitoring_permission_granted,
    is_screen_recording_permission_granted, prompt_accessibility_permission,
    prompt_input_monitoring_permission, prompt_screen_recording_permission,
};
