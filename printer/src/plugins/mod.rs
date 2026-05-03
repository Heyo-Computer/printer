pub mod cli;
pub mod exec;
pub mod install;
pub mod registry;
pub mod source;
pub mod store;

pub use cli::AddPluginArgs;
pub use exec::exec_external;
pub use install::{add_plugin, reinstall_all, reinstall_plugin};
pub use store::{list_installed, prompt_if_no_plugins};
