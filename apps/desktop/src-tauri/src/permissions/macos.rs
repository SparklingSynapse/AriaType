#![allow(unexpected_cfgs)]

use std::process::Command;

use super::{PermissionProvider, PermissionStatus};

pub struct MacosPermissions;

impl PermissionProvider for MacosPermissions {
    fn check_accessibility(&self) -> PermissionStatus {
        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            fn AXIsProcessTrusted() -> bool;
        }
        if unsafe { AXIsProcessTrusted() } {
            PermissionStatus::Granted
        } else {
            PermissionStatus::Denied
        }
    }

    fn check_input_monitoring(&self) -> PermissionStatus {
        #[link(name = "IOKit", kind = "framework")]
        extern "C" {
            fn IOHIDCheckAccess(request_type: u32) -> u32;
        }
        if unsafe { IOHIDCheckAccess(0) } == 0 {
            PermissionStatus::Granted
        } else {
            PermissionStatus::Denied
        }
    }

    fn check_microphone(&self) -> PermissionStatus {
        use objc::{class, msg_send, sel, sel_impl};

        #[link(name = "AVFoundation", kind = "framework")]
        extern "C" {}

        unsafe {
            let media_type: *mut objc::runtime::Object = msg_send![
                class!(NSString),
                stringWithUTF8String: c"soun".as_ptr()
            ];
            // AVAuthorizationStatus: 0=notDetermined, 1=restricted, 2=denied, 3=authorized
            let status: i64 = msg_send![
                class!(AVCaptureDevice),
                authorizationStatusForMediaType: media_type
            ];
            match status {
                3 => PermissionStatus::Granted,
                2 | 1 => PermissionStatus::Denied,
                _ => PermissionStatus::NotDetermined,
            }
        }
    }

    fn check_screen_recording(&self) -> PermissionStatus {
        #[link(name = "CoreGraphics", kind = "framework")]
        extern "C" {
            fn CGPreflightScreenCaptureAccess() -> bool;
        }

        if unsafe { CGPreflightScreenCaptureAccess() } {
            PermissionStatus::Granted
        } else {
            PermissionStatus::Denied
        }
    }

    fn apply_accessibility(&self) -> Result<(), String> {
        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
        }
        #[link(name = "CoreFoundation", kind = "framework")]
        extern "C" {
            fn CFDictionaryCreate(
                allocator: *const std::ffi::c_void,
                keys: *const *const std::ffi::c_void,
                values: *const *const std::ffi::c_void,
                num_values: isize,
                key_callbacks: *const std::ffi::c_void,
                value_callbacks: *const std::ffi::c_void,
            ) -> *const std::ffi::c_void;
            static kCFBooleanTrue: *const std::ffi::c_void;
            static kAXTrustedCheckOptionPrompt: *const std::ffi::c_void;
        }
        unsafe {
            let keys = [kAXTrustedCheckOptionPrompt];
            let values = [kCFBooleanTrue];
            let dict = CFDictionaryCreate(
                std::ptr::null(),
                keys.as_ptr(),
                values.as_ptr(),
                1,
                std::ptr::null(),
                std::ptr::null(),
            );
            AXIsProcessTrustedWithOptions(dict);
        }
        Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn()
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn apply_input_monitoring(&self) -> Result<(), String> {
        Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent")
            .spawn()
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn apply_microphone(&self) -> Result<(), String> {
        if self.check_microphone() == PermissionStatus::NotDetermined {
            use objc::{class, msg_send, sel, sel_impl};

            #[link(name = "AVFoundation", kind = "framework")]
            extern "C" {}

            let (tx, rx) = std::sync::mpsc::channel::<bool>();
            let tx = std::sync::Mutex::new(Some(tx));

            unsafe {
                let media_type: *mut objc::runtime::Object = msg_send![
                    class!(NSString),
                    stringWithUTF8String: c"soun".as_ptr()
                ];
                let tx_ptr = Box::into_raw(Box::new(tx));
                extern crate block;
                let block = block::ConcreteBlock::new(move |granted: objc::runtime::BOOL| {
                    let tx = Box::from_raw(tx_ptr);
                    let sender = tx.lock().ok().and_then(|mut guard| guard.take());
                    if let Some(sender) = sender {
                        let _ = sender.send(granted == objc::runtime::YES);
                    }
                });
                let block = block.copy();
                let _: () = msg_send![
                    class!(AVCaptureDevice),
                    requestAccessForMediaType: media_type
                    completionHandler: &*block
                ];
            }

            let _ = rx.recv_timeout(std::time::Duration::from_secs(60));
        } else {
            Command::new("open")
                .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")
                .spawn()
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn apply_screen_recording(&self) -> Result<(), String> {
        Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
            .spawn()
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}
