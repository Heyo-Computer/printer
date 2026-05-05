use anyhow::{Result, bail};
use evdev::KeyCode as Key;

/// Map a key name (case-insensitive) to a Linux evdev keycode.
/// Names follow wtype conventions where possible.
pub fn parse_key(name: &str) -> Result<Key> {
    let n = name.trim().to_ascii_lowercase();
    let k = match n.as_str() {
        // Letters
        "a" => Key::KEY_A, "b" => Key::KEY_B, "c" => Key::KEY_C, "d" => Key::KEY_D,
        "e" => Key::KEY_E, "f" => Key::KEY_F, "g" => Key::KEY_G, "h" => Key::KEY_H,
        "i" => Key::KEY_I, "j" => Key::KEY_J, "k" => Key::KEY_K, "l" => Key::KEY_L,
        "m" => Key::KEY_M, "n" => Key::KEY_N, "o" => Key::KEY_O, "p" => Key::KEY_P,
        "q" => Key::KEY_Q, "r" => Key::KEY_R, "s" => Key::KEY_S, "t" => Key::KEY_T,
        "u" => Key::KEY_U, "v" => Key::KEY_V, "w" => Key::KEY_W, "x" => Key::KEY_X,
        "y" => Key::KEY_Y, "z" => Key::KEY_Z,
        // Digits
        "0" => Key::KEY_0, "1" => Key::KEY_1, "2" => Key::KEY_2, "3" => Key::KEY_3,
        "4" => Key::KEY_4, "5" => Key::KEY_5, "6" => Key::KEY_6, "7" => Key::KEY_7,
        "8" => Key::KEY_8, "9" => Key::KEY_9,
        // Whitespace + control
        "space" | " " => Key::KEY_SPACE,
        "enter" | "return" => Key::KEY_ENTER,
        "tab" => Key::KEY_TAB,
        "esc" | "escape" => Key::KEY_ESC,
        "backspace" | "bs" => Key::KEY_BACKSPACE,
        "delete" | "del" => Key::KEY_DELETE,
        "insert" | "ins" => Key::KEY_INSERT,
        "home" => Key::KEY_HOME,
        "end" => Key::KEY_END,
        "pageup" | "pgup" => Key::KEY_PAGEUP,
        "pagedown" | "pgdn" => Key::KEY_PAGEDOWN,
        "up" => Key::KEY_UP,
        "down" => Key::KEY_DOWN,
        "left" => Key::KEY_LEFT,
        "right" => Key::KEY_RIGHT,
        // Modifiers
        "ctrl" | "control" | "leftctrl" => Key::KEY_LEFTCTRL,
        "rightctrl" | "rctrl" => Key::KEY_RIGHTCTRL,
        "shift" | "leftshift" => Key::KEY_LEFTSHIFT,
        "rightshift" | "rshift" => Key::KEY_RIGHTSHIFT,
        "alt" | "leftalt" => Key::KEY_LEFTALT,
        "rightalt" | "altgr" => Key::KEY_RIGHTALT,
        "super" | "meta" | "win" | "leftmeta" => Key::KEY_LEFTMETA,
        "rightmeta" | "rsuper" => Key::KEY_RIGHTMETA,
        "capslock" | "caps" => Key::KEY_CAPSLOCK,
        // Function keys
        "f1" => Key::KEY_F1, "f2" => Key::KEY_F2, "f3" => Key::KEY_F3, "f4" => Key::KEY_F4,
        "f5" => Key::KEY_F5, "f6" => Key::KEY_F6, "f7" => Key::KEY_F7, "f8" => Key::KEY_F8,
        "f9" => Key::KEY_F9, "f10" => Key::KEY_F10, "f11" => Key::KEY_F11, "f12" => Key::KEY_F12,
        // Punctuation (US layout)
        "minus" | "-" => Key::KEY_MINUS,
        "equal" | "equals" | "=" => Key::KEY_EQUAL,
        "leftbrace" | "[" => Key::KEY_LEFTBRACE,
        "rightbrace" | "]" => Key::KEY_RIGHTBRACE,
        "semicolon" | ";" => Key::KEY_SEMICOLON,
        "apostrophe" | "'" => Key::KEY_APOSTROPHE,
        "grave" | "`" => Key::KEY_GRAVE,
        "backslash" | "\\" => Key::KEY_BACKSLASH,
        "comma" | "," => Key::KEY_COMMA,
        "dot" | "period" | "." => Key::KEY_DOT,
        "slash" | "/" => Key::KEY_SLASH,
        _ => bail!("unknown key: {name}"),
    };
    Ok(k)
}

/// Returns (keycode, needs_shift) for a single ASCII char on US layout.
pub fn char_to_key(c: char) -> Option<(Key, bool)> {
    let k = match c {
        'a'..='z' => (parse_key(&c.to_string()).ok()?, false),
        'A'..='Z' => (parse_key(&c.to_ascii_lowercase().to_string()).ok()?, true),
        '0' => (Key::KEY_0, false), '1' => (Key::KEY_1, false), '2' => (Key::KEY_2, false),
        '3' => (Key::KEY_3, false), '4' => (Key::KEY_4, false), '5' => (Key::KEY_5, false),
        '6' => (Key::KEY_6, false), '7' => (Key::KEY_7, false), '8' => (Key::KEY_8, false),
        '9' => (Key::KEY_9, false),
        ' ' => (Key::KEY_SPACE, false),
        '\n' => (Key::KEY_ENTER, false),
        '\t' => (Key::KEY_TAB, false),
        '-' => (Key::KEY_MINUS, false),
        '_' => (Key::KEY_MINUS, true),
        '=' => (Key::KEY_EQUAL, false),
        '+' => (Key::KEY_EQUAL, true),
        '[' => (Key::KEY_LEFTBRACE, false),
        '{' => (Key::KEY_LEFTBRACE, true),
        ']' => (Key::KEY_RIGHTBRACE, false),
        '}' => (Key::KEY_RIGHTBRACE, true),
        ';' => (Key::KEY_SEMICOLON, false),
        ':' => (Key::KEY_SEMICOLON, true),
        '\'' => (Key::KEY_APOSTROPHE, false),
        '"' => (Key::KEY_APOSTROPHE, true),
        '`' => (Key::KEY_GRAVE, false),
        '~' => (Key::KEY_GRAVE, true),
        '\\' => (Key::KEY_BACKSLASH, false),
        '|' => (Key::KEY_BACKSLASH, true),
        ',' => (Key::KEY_COMMA, false),
        '<' => (Key::KEY_COMMA, true),
        '.' => (Key::KEY_DOT, false),
        '>' => (Key::KEY_DOT, true),
        '/' => (Key::KEY_SLASH, false),
        '?' => (Key::KEY_SLASH, true),
        '!' => (Key::KEY_1, true),
        '@' => (Key::KEY_2, true),
        '#' => (Key::KEY_3, true),
        '$' => (Key::KEY_4, true),
        '%' => (Key::KEY_5, true),
        '^' => (Key::KEY_6, true),
        '&' => (Key::KEY_7, true),
        '*' => (Key::KEY_8, true),
        '(' => (Key::KEY_9, true),
        ')' => (Key::KEY_0, true),
        _ => return None,
    };
    Some(k)
}
