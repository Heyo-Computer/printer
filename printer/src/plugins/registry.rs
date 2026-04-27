/// A bundled, well-known plugin: just `printer add-plugin <name>` works
/// without the user needing to remember a URL or installer command.
pub struct Known {
    pub name: &'static str,
    pub installer: KnownInstaller,
}

#[allow(dead_code)] // `Cargo` variant is intended for future registry entries.
pub enum KnownInstaller {
    /// Plugin is a Rust crate cloned from `git` and built with `cargo install`.
    Cargo { git: &'static str },
    /// Plugin is installed by piping a shell command (typically the vendor's
    /// own `curl … | sh` installer). After it finishes, the binary is
    /// expected at `binary` (with `~` expansion). The plugin is dispatched
    /// from there directly — printer does not move or symlink it.
    Shell {
        command: &'static str,
        binary: &'static str,
    },
}

/// One edit point — when `heyvm` (or any future plugin) goes public, fix
/// the URL/command here.
pub const REGISTRY: &[Known] = &[Known {
    name: "heyvm",
    installer: KnownInstaller::Shell {
        command: "curl -fsSL https://heyo.computer/heyvm/install.sh | sh",
        binary: "~/.local/bin/heyvm",
    },
}];

pub fn lookup(name: &str) -> Option<&'static Known> {
    REGISTRY.iter().find(|k| k.name == name)
}
