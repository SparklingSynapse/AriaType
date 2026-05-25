use cocoa::base::{id, nil};
use cocoa::foundation::NSString;
use objc::{msg_send, sel, sel_impl};
use std::collections::{HashSet, VecDeque};
use std::ffi::{c_void, CStr};
use std::os::raw::{c_char, c_uchar};
use std::ptr;

type AXUIElementRef = *const c_void;
type AXValueRef = *const c_void;
type CFStringRef = *const c_void;
type CFTypeRef = *const c_void;

const AX_ERROR_SUCCESS: i32 = 0;
const AX_VALUE_CF_RANGE_TYPE: i32 = 4;
const MAX_ACCESSIBILITY_SEARCH_NODES: usize = 128;

#[repr(C)]
#[derive(Clone, Copy)]
struct CFRange {
    location: isize,
    length: isize,
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateSystemWide() -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;
    fn AXUIElementCopyParameterizedAttributeValue(
        element: AXUIElementRef,
        parameterized_attribute: CFStringRef,
        parameter: CFTypeRef,
        value: *mut CFTypeRef,
    ) -> i32;
    fn AXValueCreate(theType: i32, valuePtr: *const c_void) -> AXValueRef;
    fn AXValueGetValue(value: AXValueRef, theType: i32, valuePtr: *mut c_void) -> c_uchar;
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
        let text = read_text_from_focused_element(focused as AXUIElementRef)
            .or_else(|| find_text_in_accessible_subtree(focused as AXUIElementRef));
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

unsafe fn read_text_from_focused_element(element: AXUIElementRef) -> Option<String> {
    read_primary_text_from_element(element)
        .or_else(|| copy_attribute_string(element, "AXSelectedText"))
        .or_else(|| copy_attribute_string(element, "AXTitle"))
        .and_then(super::non_empty_text)
}

unsafe fn read_primary_text_from_element(element: AXUIElementRef) -> Option<String> {
    copy_attribute_string(element, "AXValue")
        .or_else(|| copy_text_range_string(element))
        .and_then(super::non_empty_text)
}

unsafe fn copy_attribute_string(element: AXUIElementRef, attribute: &str) -> Option<String> {
    let value = copy_attribute_value(element, attribute)?;
    let text = ns_object_to_string(value as id);
    CFRelease(value);
    text
}

unsafe fn copy_attribute_i64(element: AXUIElementRef, attribute: &str) -> Option<i64> {
    let value = copy_attribute_value(element, attribute)?;
    let number = ns_object_to_i64(value as id);
    CFRelease(value);
    number
}

unsafe fn copy_attribute_range(element: AXUIElementRef, attribute: &str) -> Option<CFRange> {
    let value = copy_attribute_value(element, attribute)?;
    let mut range = CFRange {
        location: 0,
        length: 0,
    };
    let success = AXValueGetValue(
        value as AXValueRef,
        AX_VALUE_CF_RANGE_TYPE,
        &mut range as *mut CFRange as *mut c_void,
    ) != 0;
    CFRelease(value);

    if success {
        Some(range)
    } else {
        None
    }
}

unsafe fn copy_text_range_string(element: AXUIElementRef) -> Option<String> {
    let character_count = i32::try_from(copy_attribute_i64(element, "AXNumberOfCharacters")?)
        .ok()
        .filter(|count| *count > 0)?;
    let caret_offset = copy_attribute_range(element, "AXSelectedTextRange")
        .and_then(|range| i32::try_from(range.location).ok());
    let (start, end) = super::bounded_text_range(character_count, caret_offset)?;
    let range = CFRange {
        location: isize::try_from(start).ok()?,
        length: isize::try_from(end.saturating_sub(start)).ok()?,
    };

    copy_string_for_range(element, "AXStringForRange", range)
        .or_else(|| copy_string_for_range(element, "AXAttributedStringForRange", range))
        .and_then(super::non_empty_text)
}

unsafe fn copy_string_for_range(
    element: AXUIElementRef,
    parameterized_attribute: &str,
    range: CFRange,
) -> Option<String> {
    let attribute_name = NSString::alloc(nil).init_str(parameterized_attribute);
    let range_value = AXValueCreate(
        AX_VALUE_CF_RANGE_TYPE,
        &range as *const CFRange as *const c_void,
    );
    if range_value.is_null() {
        let _: () = msg_send![attribute_name, release];
        return None;
    }

    let mut value: CFTypeRef = ptr::null();
    let error = AXUIElementCopyParameterizedAttributeValue(
        element,
        attribute_name as CFStringRef,
        range_value as CFTypeRef,
        &mut value,
    );
    let _: () = msg_send![attribute_name, release];
    CFRelease(range_value as CFTypeRef);

    if error != AX_ERROR_SUCCESS || value.is_null() {
        return None;
    }

    let text = ns_object_to_string(value as id);
    CFRelease(value);
    text
}

unsafe fn find_text_in_accessible_subtree(root: AXUIElementRef) -> Option<String> {
    let mut queue = VecDeque::from(copy_accessible_children(root));
    let mut visited = HashSet::new();

    while let Some(element) = queue.pop_front() {
        if visited.len() >= MAX_ACCESSIBILITY_SEARCH_NODES {
            release_elements(queue);
            CFRelease(element as CFTypeRef);
            return None;
        }

        let address = element as usize;
        if !visited.insert(address) {
            CFRelease(element as CFTypeRef);
            continue;
        }

        if let Some(text) = read_primary_text_from_element(element) {
            release_elements(queue);
            CFRelease(element as CFTypeRef);
            return Some(text);
        }

        queue.extend(copy_accessible_children(element));
        CFRelease(element as CFTypeRef);
    }

    None
}

unsafe fn copy_accessible_children(element: AXUIElementRef) -> Vec<AXUIElementRef> {
    let Some(value) = copy_attribute_value(element, "AXChildren") else {
        return Vec::new();
    };

    let array = value as id;
    let responds_to_count: bool = msg_send![array, respondsToSelector: sel!(count)];
    let responds_to_object: bool = msg_send![array, respondsToSelector: sel!(objectAtIndex:)];
    if !responds_to_count || !responds_to_object {
        CFRelease(value);
        return Vec::new();
    }

    let count: usize = msg_send![array, count];
    let remaining_capacity = MAX_ACCESSIBILITY_SEARCH_NODES.min(count);
    let mut children = Vec::with_capacity(remaining_capacity);

    for index in 0..remaining_capacity {
        let child: id = msg_send![array, objectAtIndex: index];
        if child.is_null() {
            continue;
        }

        let retained_child: id = msg_send![child, retain];
        children.push(retained_child as AXUIElementRef);
    }

    CFRelease(value);
    children
}

unsafe fn release_elements(elements: VecDeque<AXUIElementRef>) {
    for element in elements {
        CFRelease(element as CFTypeRef);
    }
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

unsafe fn ns_object_to_i64(value: id) -> Option<i64> {
    if value.is_null() {
        return None;
    }

    let responds: bool = msg_send![value, respondsToSelector: sel!(longLongValue)];
    if !responds {
        return None;
    }

    let number: i64 = msg_send![value, longLongValue];
    Some(number)
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
