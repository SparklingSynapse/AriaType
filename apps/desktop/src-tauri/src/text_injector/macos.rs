#![allow(deprecated)]
use cocoa::base::{id, nil};
use cocoa::foundation::NSString;
use enigo::{Enigo, Keyboard, Settings};
use objc::{class, msg_send, sel, sel_impl};
use std::ffi::c_void;
use std::process::Command;
use tracing::{info, warn};

pub struct MacosInjector;

// ── GCD ───────────────────────────────────────────────────────────────────────

type DispatchQueue = *const c_void;
type DispatchTime = u64;
const DISPATCH_TIME_NOW: DispatchTime = 0;
const NSEC_PER_MSEC: u64 = 1_000_000;

extern "C" {
    static _dispatch_main_q: u8;
    fn dispatch_time(when: DispatchTime, delta: i64) -> DispatchTime;
    fn dispatch_after_f(
        when: DispatchTime,
        queue: DispatchQueue,
        context: *mut c_void,
        work: unsafe extern "C" fn(*mut c_void),
    );
    fn dispatch_sync_f(
        queue: DispatchQueue,
        context: *mut c_void,
        work: unsafe extern "C" fn(*mut c_void),
    );
}

unsafe fn schedule_on_main<F: FnOnce() + Send + 'static>(delay_ms: u64, f: F) {
    extern "C" fn trampoline<F: FnOnce()>(ctx: *mut c_void) {
        let f = unsafe { Box::from_raw(ctx as *mut F) };
        f();
    }
    let ctx = Box::into_raw(Box::new(f)) as *mut c_void;
    let main_queue = &_dispatch_main_q as *const u8 as DispatchQueue;
    let when = dispatch_time(DISPATCH_TIME_NOW, (delay_ms * NSEC_PER_MSEC) as i64);
    dispatch_after_f(when, main_queue, ctx, trampoline::<F>);
}

unsafe fn run_on_main_sync<T: Send, F: FnOnce() -> T + Send>(f: F) -> T {
    struct Ctx<T, F> {
        f: Option<F>,
        result: Option<T>,
    }
    extern "C" fn trampoline<T, F: FnOnce() -> T>(ctx: *mut c_void) {
        unsafe {
            let ctx = &mut *(ctx as *mut Ctx<T, F>);
            if let Some(f) = ctx.f.take() {
                ctx.result = Some(f());
            }
        }
    }
    let mut ctx = Ctx {
        f: Some(f),
        result: None,
    };
    let main_queue = &_dispatch_main_q as *const u8 as DispatchQueue;
    dispatch_sync_f(
        main_queue,
        &mut ctx as *mut _ as *mut c_void,
        trampoline::<T, F>,
    );
    ctx.result.expect("run_on_main_sync failed")
}

// ── NSPasteboard save/restore ─────────────────────────────────────────────────

#[derive(Copy, Clone)]
enum SavedValueKind {
    Plist,
    Data,
    String,
}

struct SavedItemType {
    ty: id,
    value: id,
    kind: SavedValueKind,
}
struct SavedItem {
    types: Vec<SavedItemType>,
}
struct SavedItems {
    items: Vec<SavedItem>,
}
unsafe impl Send for SavedItems {}

impl Drop for SavedItems {
    fn drop(&mut self) {
        for item in &mut self.items {
            for t in &mut item.types {
                unsafe {
                    if !t.ty.is_null() {
                        let _: () = msg_send![t.ty, release];
                    }
                    if !t.value.is_null() {
                        let _: () = msg_send![t.value, release];
                    }
                }
            }
        }
    }
}

unsafe fn pb_save() -> Option<SavedItems> {
    let pb: id = msg_send![class!(NSPasteboard), generalPasteboard];
    let items: id = msg_send![pb, pasteboardItems];
    let count: usize = msg_send![items, count];
    if count == 0 {
        return None;
    }
    let mut saved_items = Vec::with_capacity(count);
    for index in 0..count {
        let item: id = msg_send![items, objectAtIndex: index];
        let types: id = msg_send![item, types];
        let type_count: usize = msg_send![types, count];
        let mut saved_types = Vec::with_capacity(type_count);
        for type_index in 0..type_count {
            let ty: id = msg_send![types, objectAtIndex: type_index];
            let plist: id = msg_send![item, propertyListForType: ty];
            if !plist.is_null() {
                let _: () = msg_send![ty, retain];
                let _: () = msg_send![plist, retain];
                saved_types.push(SavedItemType {
                    ty,
                    value: plist,
                    kind: SavedValueKind::Plist,
                });
                continue;
            }
            let data: id = msg_send![item, dataForType: ty];
            if !data.is_null() {
                let _: () = msg_send![ty, retain];
                let _: () = msg_send![data, retain];
                saved_types.push(SavedItemType {
                    ty,
                    value: data,
                    kind: SavedValueKind::Data,
                });
                continue;
            }
            let string: id = msg_send![item, stringForType: ty];
            if string.is_null() {
                continue;
            }
            let _: () = msg_send![ty, retain];
            let _: () = msg_send![string, retain];
            saved_types.push(SavedItemType {
                ty,
                value: string,
                kind: SavedValueKind::String,
            });
        }
        if !saved_types.is_empty() {
            saved_items.push(SavedItem { types: saved_types });
        }
    }
    if saved_items.is_empty() {
        None
    } else {
        Some(SavedItems { items: saved_items })
    }
}

unsafe fn pb_write_text(text: &str) {
    let pb: id = msg_send![class!(NSPasteboard), generalPasteboard];
    let _: () = msg_send![pb, clearContents];
    let ns_str = NSString::alloc(nil).init_str(text);
    let ns_type = NSString::alloc(nil).init_str("public.utf8-plain-text");
    let _: bool = msg_send![pb, setString: ns_str forType: ns_type];
    let _: () = msg_send![ns_str, release];
    let _: () = msg_send![ns_type, release];
}

unsafe fn pb_restore(saved: &SavedItems) {
    let pb: id = msg_send![class!(NSPasteboard), generalPasteboard];
    let _: () = msg_send![pb, clearContents];
    let array: id = msg_send![class!(NSMutableArray), array];
    for saved_item in &saved.items {
        let item: id = msg_send![class!(NSPasteboardItem), alloc];
        let item: id = msg_send![item, init];
        for t in &saved_item.types {
            match t.kind {
                SavedValueKind::Plist => {
                    let _: bool = msg_send![item, setPropertyList: t.value forType: t.ty];
                }
                SavedValueKind::Data => {
                    let _: bool = msg_send![item, setData: t.value forType: t.ty];
                }
                SavedValueKind::String => {
                    let _: bool = msg_send![item, setString: t.value forType: t.ty];
                }
            }
        }
        let _: () = msg_send![array, addObject: item];
        let _: () = msg_send![item, release];
    }
    let _: bool = msg_send![pb, writeObjects: array];
}

// ── Layers ────────────────────────────────────────────────────────────────────

/// Layer 0: enigo key_sequence — simulates keyboard input, never touches the clipboard.
/// Best for short text without special formatting. Fast but can lose characters with long text.
fn try_enigo_key_sequence(text: &str) -> bool {
    info!(
        text_len = text.len(),
        "inject[L0]: trying enigo key_sequence"
    );
    let mut enigo = match Enigo::new(&Settings::default()) {
        Ok(e) => e,
        Err(e) => {
            warn!("inject[L0]: failed to create Enigo: {e}");
            return false;
        }
    };
    match enigo.text(text) {
        Ok(_) => {
            info!("inject[L0]: succeeded");
            true
        }
        Err(e) => {
            warn!("inject[L0]: failed: {e}");
            false
        }
    }
}

/// Layer 2: write text to clipboard then simulate Cmd+V via osascript.
/// Saves and restores all pasteboard content so nothing is lost.
/// More reliable for long text, multiline text, and special characters.
fn try_clipboard_paste(text: &str) -> bool {
    let saved = unsafe { run_on_main_sync(|| pb_save()) };
    unsafe { run_on_main_sync(|| pb_write_text(text)) };
    info!("inject[L2]: clipboard written");

    let script = "tell application \"System Events\" to keystroke \"v\" using command down";
    let ok = match Command::new("osascript").args(["-e", script]).output() {
        Ok(out) if out.status.success() => {
            info!("inject[L2]: osascript succeeded");
            true
        }
        Ok(out) => {
            warn!(exit_code = out.status.code(), stderr = %String::from_utf8_lossy(&out.stderr).trim(), "inject[L2]: osascript failed");
            false
        }
        Err(e) => {
            warn!(error = %e, "inject[L2]: failed to spawn osascript");
            false
        }
    };

    unsafe {
        schedule_on_main(500, move || match saved {
            Some(items) => {
                pb_restore(&items);
                info!("inject[L2]: clipboard restored");
            }
            None => {
                let pb: id = msg_send![class!(NSPasteboard), generalPasteboard];
                let _: () = msg_send![pb, clearContents];
                info!("inject[L2]: clipboard cleared (was empty)");
            }
        });
    }
    ok
}

impl super::TextInjector for MacosInjector {
    fn insert(&self, text: &str, _write_clipboard: &dyn Fn()) {
        info!(text_len = text.len(), "inject: starting injection");

        // For long text (>400 chars) or text with newlines, use clipboard paste directly
        // to avoid character loss from fast keyboard simulation
        if text.len() > 400 {
            info!("inject: using L2 (clipboard paste) for long/multiline text");
            let ok = try_clipboard_paste(text);
            info!(success = ok, "inject: done via L2");
            return;
        }

        if try_enigo_key_sequence(text) {
            info!("inject: done via L0 (enigo)");
            return;
        }
        info!("inject: L0 failed, falling through to L2 (clipboard paste)");
        let ok = try_clipboard_paste(text);
        info!(success = ok, "inject: done via L2");
    }
}
