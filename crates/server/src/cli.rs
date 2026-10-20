//! The command line. Owner commands (Task 12) are the emergency switch of spec §7.4: they run
//! against the production database from the owner's machine or Cloud Shell, never over HTTP.

use clap::{Parser, Subcommand};
use uuid::Uuid;

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
    /// Show an offer's price, history and pending reports.
    ShowOffer { offer: String },
    /// Stop all reports and votes on an offer (abuse only).
    LockOffer { offer: String },
    /// Allow reports and votes on an offer again.
    UnlockOffer { offer: String },
    /// Remove an offer from the catalogue and stop all crowd input on it (abuse only).
    HideOffer { offer: String },
    /// Show a hidden offer again.
    UnhideOffer { offer: String },
    /// Close a pending report or proposal without deciding it; nobody is settled (abuse only).
    CloseReport { report: Uuid },
    /// Put back the offer's previous price and close its pending reports (abuse only).
    RevertOffer { offer: String },
    /// Route between two Berlin landmarks through OpenRouteService, to check the key, TLS and
    /// the network from inside the deployed image. Needs ORS_API_KEY; no database.
    CheckRouting,
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

    #[test]
    fn check_routing_parses() {
        assert_eq!(parse(&["check-routing"]).unwrap(), Command::CheckRouting);
    }

    #[test]
    fn owner_commands_parse() {
        assert_eq!(
            parse(&["lock-offer", "bolt.corsa"]).unwrap(),
            Command::LockOffer {
                offer: "bolt.corsa".into()
            }
        );
        assert_eq!(
            parse(&["revert-offer", "bolt.corsa"]).unwrap(),
            Command::RevertOffer {
                offer: "bolt.corsa".into()
            }
        );
        assert!(parse(&["lock-offer"]).is_err());
        assert_eq!(
            parse(&["hide-offer", "bolt.spam"]).unwrap(),
            Command::HideOffer {
                offer: "bolt.spam".into()
            }
        );
        assert_eq!(
            parse(&["unhide-offer", "bolt.spam"]).unwrap(),
            Command::UnhideOffer {
                offer: "bolt.spam".into()
            }
        );
        let report = "123e4567-e89b-42d3-a456-426614174000";
        assert_eq!(
            parse(&["close-report", report]).unwrap(),
            Command::CloseReport {
                report: report.parse().unwrap()
            }
        );
        assert!(parse(&["close-report", "not-a-uuid"]).is_err());
    }
}
