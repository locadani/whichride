//! The command line. Owner commands (Task 12) are the emergency switch of spec §7.4: they run
//! against the production database from the owner's machine or Cloud Shell, never over HTTP.

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "whichride-server",
    version,
    about = "whichride's server and the owner's emergency switch"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// Apply migrations, add new offers from the seed catalogue, then serve on BIND_ADDR.
    Serve,
    /// Apply migrations and add new offers from the seed catalogue, then exit.
    Migrate,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    fn parse(args: &[&str]) -> Result<Command, clap::Error> {
        Cli::try_parse_from(std::iter::once("whichride-server").chain(args.iter().copied()))
            .map(|cli| cli.command)
    }

    #[test]
    fn the_command_line_is_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn every_command_parses() {
        assert_eq!(parse(&["serve"]).unwrap(), Command::Serve);
        assert_eq!(parse(&["migrate"]).unwrap(), Command::Migrate);
        assert!(parse(&["unknown"]).is_err());
    }
}
