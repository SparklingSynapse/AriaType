#![allow(unexpected_cfgs)]
use std::process::Command;

pub struct MacosPermissions;

impl super::PermissionProvider for MacosPermissions {
    fn check_accessibility(&self) -> String {
        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            fn AXIsProcessTrusted() -> bool;
        }
        if unsafe { AXIsProcessTrusted() } { "granted".to_string() } else { "denied".to_string() }
    }

    fn check_input_monitoring(&self) -> String {
        #[link(name = "IOKit", kind = "framework")]
        extern "C" {
            fn IOHIDCheckAccess(request_type: u32) -> u32;
        }
        if unsafe { IOHIDCheckAccess(0) } == 0 { "granted".to_string() } else { "denied".to_string() }
    }

    fn check_microphone(&self) -> String {
        use objc::{class, msg_send, sel, sel_impl};
        use std::os::raw::c_char;
        #[link(name = "AVFoundation", kind = "framework")]
        extern "C" {}
        unsafe {
            let media_type: *mut objc::runtime::Object = msg_send![
                class!(NSString),
                stringWithUTF8String: b"soun\0".as_ptr() as *const c_char
            ];
            // AVAuthorizationStatus: 0=notDetermined, 1=restricted, 2=denied, 3=authorized
            let status: i64 = msg_send![
                class!(AVCaptureDevice),
                authorizationStatusForMediaType: media_type
            ];
            match status {
                3 => "granted".to_string(),
                2 | 1 => "denied".to_string(),
                _ => "not_determined".to_string(),
            }
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
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn apply_input_monitoring(&self) -> Result<(), String> {
        Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent")
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn apply_microphone(&self) -> Result<(), String> {
        if self.check_microphone() == "not_determined" {
            use objc::{class, msg_send, sel, sel_impl};
            use std::os::raw::c_char;
            #[link(name = "AVFoundation", kind = "framework")]
            extern "C" {}
            let (tx, rx) = std::sync::mpsc::channel::<bool>();
            let tx = std::sync::Mutex::new(Some(tx));
            unsafe {
                let media_type: *mut objc::runtime::Object = msg_send![
                    class!(NSString),
                    stringWithUTF8String: b"soun\0".as_ptr() as *const c_char
                ];
                let tx_ptr = Box::into_raw(Box::new(tx));
                extern crate block;
                let block = block::ConcreteBlock::new(move |granted: objc::runtime::BOOL| {
                    let tx = Box::from_raw(tx_ptr);
                    if let Ok(mut guard) = tx.lock() {
                        if let Some(sender) = guard.take() {
                            let _ = sender.send(granted == objc::runtime::YES);
                        }
                    };
                });
                let block = block.copy();
                let _: () = msg_send![
                    class!(AVCaptureDevice),
                    requestAccessForMediaType: media_type
                    completionHandler: &*block
                ];
            }
            // Wait up to 60 s for the user to respond to the dialog
            let _ = rx.recv_timeout(std::time::Duration::from_secs(60));
        } else {
            Command::new("open")
                .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
