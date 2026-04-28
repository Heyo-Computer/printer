use anyhow::{Result, bail};
use core_graphics::event::{CGEventFlags, CGKeyCode, KeyCode as Vk};

pub fn parse_key(name: &str) -> Result<CGKeyCode> {
    let n = name.trim().to_ascii_lowercase();
    let k = match n.as_str() {
        // Letters
        "a" => Vk::ANSI_A, "b" => Vk::ANSI_B, "c" => Vk::ANSI_C, "d" => Vk::ANSI_D,
        "e" => Vk::ANSI_E, "f" => Vk::ANSI_F, "g" => Vk::ANSI_G, "h" => Vk::ANSI_H,
        "i" => Vk::ANSI_I, "j" => Vk::ANSI_J, "k" => Vk::ANSI_K, "l" => Vk::ANSI_L,
        "m" => Vk::ANSI_M, "n" => Vk::ANSI_N, "o" => Vk::ANSI_O, "p" => Vk::ANSI_P,
        "q" => Vk::ANSI_Q, "r" => Vk::ANSI_R, "s" => Vk::ANSI_S, "t" => Vk::ANSI_T,
        "u" => Vk::ANSI_U, "v" => Vk::ANSI_V, "w" => Vk::ANSI_W, "x" => Vk::ANSI_X,
        "y" => Vk::ANSI_Y, "z" => Vk::ANSI_Z,
        // Digits
        "0" => Vk::ANSI_0, "1" => Vk::ANSI_1, "2" => Vk::ANSI_2, "3" => Vk::ANSI_3,
        "4" => Vk::ANSI_4, "5" => Vk::ANSI_5, "6" => Vk::ANSI_6, "7" => Vk::ANSI_7,
        "8" => Vk::ANSI_8, "9" => Vk::ANSI_9,
        // Whitespace + control
        "space" | " " => Vk::SPACE,
        "enter" | "return" => Vk::RETURN,
        "tab" => Vk::TAB,
        "esc" | "escape" => Vk::ESCAPE,
        // Apple's "Delete" key (above Return) is what Linux calls Backspace.
        "backspace" | "bs" => Vk::DELETE,
        "delete" | "del" | "forward_delete" => Vk::FORWARD_DELETE,
        "home" => Vk::HOME,
        "end" => Vk::END,
        "pageup" | "pgup" => Vk::PAGE_UP,
        "pagedown" | "pgdn" => Vk::PAGE_DOWN,
        "up" => Vk::UP_ARROW,
        "down" => Vk::DOWN_ARROW,
        "left" => Vk::LEFT_ARROW,
        "right" => Vk::RIGHT_ARROW,
        // Modifiers — kept under Linux-style names so chords are portable.
        "ctrl" | "control" | "leftctrl" => Vk::CONTROL,
        "rightctrl" | "rctrl" => Vk::RIGHT_CONTROL,
        "shift" | "leftshift" => Vk::SHIFT,
        "rightshift" | "rshift" => Vk::RIGHT_SHIFT,
        "alt" | "leftalt" | "option" | "opt" => Vk::OPTION,
        "rightalt" | "altgr" | "rightoption" => Vk::RIGHT_OPTION,
        "super" | "meta" | "win" | "leftmeta" | "cmd" | "command" => Vk::COMMAND,
        "rightmeta" | "rsuper" | "rightcmd" => Vk::RIGHT_COMMAND,
        "capslock" | "caps" => Vk::CAPS_LOCK,
        "fn" | "function" => Vk::FUNCTION,
        // Function keys
        "f1" => Vk::F1, "f2" => Vk::F2, "f3" => Vk::F3, "f4" => Vk::F4,
        "f5" => Vk::F5, "f6" => Vk::F6, "f7" => Vk::F7, "f8" => Vk::F8,
        "f9" => Vk::F9, "f10" => Vk::F10, "f11" => Vk::F11, "f12" => Vk::F12,
        // Punctuation (US layout)
        "minus" | "-" => Vk::ANSI_MINUS,
        "equal" | "equals" | "=" => Vk::ANSI_EQUAL,
        "leftbrace" | "[" => Vk::ANSI_LEFT_BRACKET,
        "rightbrace" | "]" => Vk::ANSI_RIGHT_BRACKET,
        "semicolon" | ";" => Vk::ANSI_SEMICOLON,
        "apostrophe" | "'" | "quote" => Vk::ANSI_QUOTE,
        "grave" | "`" => Vk::ANSI_GRAVE,
        "backslash" | "\\" => Vk::ANSI_BACKSLASH,
        "comma" | "," => Vk::ANSI_COMMA,
        "dot" | "period" | "." => Vk::ANSI_PERIOD,
        "slash" | "/" => Vk::ANSI_SLASH,
        _ => bail!("unknown key: {name}"),
    };
    Ok(k)
}

/// If the key name corresponds to a modifier, returns its CGEventFlags bit.
pub fn modifier_flag(keycode: CGKeyCode) -> Option<CGEventFlags> {
    match keycode {
        Vk::SHIFT | Vk::RIGHT_SHIFT => Some(CGEventFlags::CGEventFlagShift),
        Vk::CONTROL | Vk::RIGHT_CONTROL => Some(CGEventFlags::CGEventFlagControl),
        Vk::OPTION | Vk::RIGHT_OPTION => Some(CGEventFlags::CGEventFlagAlternate),
        Vk::COMMAND | Vk::RIGHT_COMMAND => Some(CGEventFlags::CGEventFlagCommand),
        Vk::CAPS_LOCK => Some(CGEventFlags::CGEventFlagAlphaShift),
        Vk::FUNCTION => Some(CGEventFlags::CGEventFlagSecondaryFn),
        _ => None,
    }
}
