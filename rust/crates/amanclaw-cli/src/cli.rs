use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "amanclaw", version, about = "Modular AI assistant for communities")]
pub struct Cli {
    /// Path to config file
    #[arg(short, long, default_value = "config.yaml")]
    pub config: String,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start the bot (default if no subcommand)
    Run,
    /// Initialize a new AmanClaw project
    Init,
    /// Start in development mode with mock LLM
    Dev,
    /// Validate config file
    Check,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_no_args_defaults_to_none_command() {
        let cli = Cli::parse_from(["amanclaw"]);
        assert!(cli.command.is_none());
        assert_eq!(cli.config, "config.yaml");
    }

    #[test]
    fn test_cli_run_subcommand() {
        let cli = Cli::parse_from(["amanclaw", "run"]);
        assert!(matches!(cli.command, Some(Command::Run)));
    }

    #[test]
    fn test_cli_init_subcommand() {
        let cli = Cli::parse_from(["amanclaw", "init"]);
        assert!(matches!(cli.command, Some(Command::Init)));
    }

    #[test]
    fn test_cli_dev_subcommand() {
        let cli = Cli::parse_from(["amanclaw", "dev"]);
        assert!(matches!(cli.command, Some(Command::Dev)));
    }

    #[test]
    fn test_cli_check_subcommand() {
        let cli = Cli::parse_from(["amanclaw", "check"]);
        assert!(matches!(cli.command, Some(Command::Check)));
    }

    #[test]
    fn test_cli_custom_config() {
        let cli = Cli::parse_from(["amanclaw", "-c", "my-config.yaml"]);
        assert_eq!(cli.config, "my-config.yaml");
    }
}
