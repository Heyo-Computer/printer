use clap::Args;

#[derive(Args, Debug)]
pub struct AddPluginArgs {
    /// Plugin spec. One of:
    ///   * a registered name (`heyvm`)
    ///   * a git URL (https://, git@..., or anything `git clone` accepts)
    ///   * a local path prefixed with `path:` (e.g. `path:/home/me/dev/heyvm`)
    pub spec: String,

    /// Override the inferred plugin name (defaults to the repo basename or
    /// the source crate's package name). Required when --install-cmd is
    /// used without a registry name.
    #[arg(long)]
    pub name: Option<String>,

    /// Pin a git ref (branch, tag, or commit). Ignored for `path:` and
    /// shell-installer sources.
    #[arg(long)]
    pub rev: Option<String>,

    /// Run this shell command as the installer instead of cloning + cargo
    /// install. Use together with --binary so printer knows where the
    /// command landed the executable. Example:
    ///   printer add-plugin heyvm \
    ///     --install-cmd "curl -fsSL https://heyo.computer/heyvm/install.sh | sh" \
    ///     --binary ~/.local/bin/heyvm
    #[arg(long = "install-cmd", value_name = "SHELL_COMMAND")]
    pub install_cmd: Option<String>,

    /// Path (with `~` expansion) where the resulting executable lives.
    /// Required with --install-cmd. Used to dispatch `printer <name>`.
    #[arg(long, value_name = "PATH")]
    pub binary: Option<String>,

    /// Reinstall over an existing plugin of the same name. Without this,
    /// `add-plugin` refuses to clobber an installed plugin.
    #[arg(long, default_value_t = false)]
    pub force: bool,
}
