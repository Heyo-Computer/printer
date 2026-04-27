mod agent;
mod cli;
mod init;
mod prompts;
mod review;
mod run;
mod session;
mod tasks;

use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    install_force_exit_supervisor();
    let cli = cli::Cli::parse();
    match cli.command {
        cli::Command::Init(args) => init::init(args),
        cli::Command::Run(args) => run::run(args).await,
        cli::Command::Review(args) => review::review(args).await,
        cli::Command::Task(args) => tasks::dispatch(args),
    }
}

/// Spawn a background task that watches Ctrl-C. The first press is just a
/// hint (the per-turn handler is expected to do the graceful shutdown). The
/// second press calls `std::process::exit(130)` so the user can always force
/// the CLI to quit, even if a child agent or some cleanup path is wedged.
fn install_force_exit_supervisor() {
    tokio::spawn(async {
        let mut count: u32 = 0;
        loop {
            if tokio::signal::ctrl_c().await.is_err() {
                return;
            }
            count += 1;
            if count == 1 {
                eprintln!("\n[printer] Ctrl-C received; cleaning up — press again to force quit");
            } else {
                eprintln!("\n[printer] forced exit");
                std::process::exit(130);
            }
        }
    });
}
