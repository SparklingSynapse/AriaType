use cocoa::base::{id, nil};
use cocoa::foundation::NSString;
use objc::{msg_send, sel, sel_impl};
use std::ffi::{c_void, CStr};
use std::os::raw::c_char;
use std::ptr;

type AXUIElementRef = *const c_void;
type CFStringRef = *const c_void;
type CFTypeRef = *const c_void;

const AX_ERROR_SUCCESS: i32 = 0;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateSystemWide() -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(cf: CFTypeRef);
}

pub fn read_focused_editable_text() -> Option<String> {
    unsafe {
        let system_wide = AXUIElementCreateSystemWide();
        if system_wide.is_null() {
            return None;
        }

        let focused = copy_attribute_value(system_wide, "AXFocusedUIElement");
        CFRelease(system_wide as CFTypeRef);

        let focused = focused?;
        let text = copy_attribute_string(focused as AXUIElementRef, "AXValue")
            .or_else(|| copy_attribute_string(focused as AXUIElementRef, "AXSelectedText"))
            .or_else(|| copy_attribute_string(focused as AXUIElementRef, "AXTitle"))
            .and_then(super::non_empty_text);
        CFRelease(focused);
        text
    }
}

unsafe fn copy_attribute_value(element: AXUIElementRef, attribute: &str) -> Option<CFTypeRef> {
    let attribute_name = NSString::alloc(nil).init_str(attribute);
    let mut value: CFTypeRef = ptr::null();
    let error = AXUIElementCopyAttributeValue(element, attribute_name as CFStringRef, &mut value);
    let _: () = msg_send![attribute_name, release];

    if error != AX_ERROR_SUCCESS || value.is_null() {
        return None;
    }

    Some(value)
}

unsafe fn copy_attribute_string(element: AXUIElementRef, attribute: &str) -> Option<String> {
    let value = copy_attribute_value(element, attribute)?;
    let text = ns_object_to_string(value as id);
    CFRelease(value);
    text
}

unsafe fn ns_object_to_string(value: id) -> Option<String> {
    ns_string_to_string(value).or_else(|| {
        let responds: bool = msg_send![value, respondsToSelector: sel!(string)];
        if !responds {
            return None;
        }

        let string_value: id = msg_send![value, string];
        ns_string_to_string(string_value)
    })
}

unsafe fn ns_string_to_string(value: id) -> Option<String> {
    if value.is_null() {
        return None;
    }

    let responds: bool = msg_send![value, respondsToSelector: sel!(UTF8String)];
    if !responds {
        return None;
    }

    let utf8: *const c_char = msg_send![value, UTF8String];
    if utf8.is_null() {
        return None;
    }

    CStr::from_ptr(utf8).to_str().ok().map(str::to_string)
}
