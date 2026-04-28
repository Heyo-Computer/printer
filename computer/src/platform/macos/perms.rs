use anyhow::{Result, bail};
use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::string::{CFString, CFStringRef};

pub fn require_accessibility() -> Result<()> {
    if is_accessibility_trusted(true) {
        return Ok(());
    }
    bail!(
        "accessibility permission required for input synthesis.\n\
         grant `computer` access in System Settings → Privacy & Security → Accessibility,\n\
         then re-run the command. (after rebuilds, codesign with `codesign --force --sign - <path>`\n\
         to keep a stable identity so the prompt isn't shown again.)"
    );
}

pub fn require_screen_recording() -> Result<()> {
    if has_screen_recording() {
        return Ok(());
    }
    // Trigger the system prompt the first time so the user is offered the toggle.
    unsafe { CGRequestScreenCaptureAccess() };
    if has_screen_recording() {
        return Ok(());
    }
    bail!(
        "screen recording permission required.\n\
         grant `computer` access in System Settings → Privacy & Security → Screen Recording,\n\
         then re-run the command."
    );
}

pub fn warn_if_screen_recording_missing() -> bool {
    let ok = has_screen_recording();
    if !ok {
        eprintln!(
            "warning: screen recording permission not granted; window titles will be empty.\n\
             grant access in System Settings → Privacy & Security → Screen Recording."
        );
    }
    ok
}

fn has_screen_recording() -> bool {
    unsafe { CGPreflightScreenCaptureAccess() }
}

fn is_accessibility_trusted(prompt: bool) -> bool {
    let prompt_key = unsafe { CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt) };
    let prompt_val = if prompt {
        CFBoolean::true_value()
    } else {
        CFBoolean::false_value()
    };
    let dict = CFDictionary::from_CFType_pairs(&[(prompt_key, prompt_val)]);
    unsafe { AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef()) }
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
    static kAXTrustedCheckOptionPrompt: CFStringRef;
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}
