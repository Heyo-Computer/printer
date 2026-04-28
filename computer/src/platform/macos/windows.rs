use anyhow::Result;
use core_foundation::array::{CFArrayGetCount, CFArrayGetValueAtIndex};
use core_foundation::base::TCFType;
use core_foundation::dictionary::{CFDictionaryGetValueIfPresent, CFDictionaryRef};
use core_foundation::number::{CFNumber, CFNumberRef};
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::display::CGDisplay;
use core_graphics::window::{
    kCGNullWindowID, kCGWindowListExcludeDesktopElements, kCGWindowListOptionOnScreenOnly,
    kCGWindowName, kCGWindowNumber, kCGWindowOwnerName,
};
use serde::Serialize;
use std::ffi::c_void;
use std::ptr;

#[derive(Serialize, Debug, Clone, Default)]
pub struct WindowInfo {
    pub identifier: String,
    pub title: String,
    pub app_id: String,
}

pub fn collect() -> Result<Vec<WindowInfo>> {
    let _ = super::perms::warn_if_screen_recording_missing();

    let opts = kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements;
    let arr = match CGDisplay::window_list_info(opts, Some(kCGNullWindowID)) {
        Some(a) => a,
        None => return Ok(Vec::new()),
    };
    let arr_ref = arr.as_concrete_TypeRef();

    let count = unsafe { CFArrayGetCount(arr_ref) };
    let mut out = Vec::with_capacity(count as usize);
    for i in 0..count {
        let dict_ptr = unsafe { CFArrayGetValueAtIndex(arr_ref, i) } as CFDictionaryRef;
        if dict_ptr.is_null() {
            continue;
        }
        let identifier = unsafe { read_number(dict_ptr, kCGWindowNumber) }
            .map(|n| n.to_string())
            .unwrap_or_default();
        let title = unsafe { read_string(dict_ptr, kCGWindowName) }.unwrap_or_default();
        let app_id = unsafe { read_string(dict_ptr, kCGWindowOwnerName) }.unwrap_or_default();
        if identifier.is_empty() && title.is_empty() && app_id.is_empty() {
            continue;
        }
        out.push(WindowInfo { identifier, title, app_id });
    }
    out.sort_by(|a, b| a.app_id.cmp(&b.app_id).then(a.title.cmp(&b.title)));
    Ok(out)
}

unsafe fn read_string(dict: CFDictionaryRef, key: CFStringRef) -> Option<String> {
    let mut value: *const c_void = ptr::null();
    let found = unsafe {
        CFDictionaryGetValueIfPresent(dict, key as *const c_void, &mut value as *mut *const c_void)
    };
    if found == 0 || value.is_null() {
        return None;
    }
    let s = unsafe { CFString::wrap_under_get_rule(value as CFStringRef) };
    Some(s.to_string())
}

unsafe fn read_number(dict: CFDictionaryRef, key: CFStringRef) -> Option<i64> {
    let mut value: *const c_void = ptr::null();
    let found = unsafe {
        CFDictionaryGetValueIfPresent(dict, key as *const c_void, &mut value as *mut *const c_void)
    };
    if found == 0 || value.is_null() {
        return None;
    }
    let n = unsafe { CFNumber::wrap_under_get_rule(value as CFNumberRef) };
    n.to_i64()
}

pub fn run(json: bool) -> Result<()> {
    let windows = collect()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&windows)?);
    } else if windows.is_empty() {
        println!("(no windows reported)");
    } else {
        for w in &windows {
            println!("{}\t{}\t{}", w.identifier, w.app_id, w.title);
        }
    }
    Ok(())
}
