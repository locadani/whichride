use std::process::ExitCode;

use clap::Parser;
use whichride_server::cli::{Cli, Command};

#[tokio::main]
async fn main() -> ExitCode {
    init_logging();
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Serve => whichride_server::commands::serve().await,
        Command::Migrate => whichride_server::commands::migrate().await,
        Command::ShowOffer { offer } => whichride_server::commands::show_offer(&offer).await,
        Command::LockOffer { offer } => whichride_server::commands::set_locked(&offer, true).await,
        Command::UnlockOffer { offer } => {
            whichride_server::commands::set_locked(&offer, false).await
        }
        Command::RevertOffer { offer } => whichride_server::commands::revert_offer(&offer).await,
        Command::CheckRouting => whichride_server::commands::check_routing().await,
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn init_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stderr()))
        .with_writer(std::io::stderr)
        .init();
}
