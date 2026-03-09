use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "amanclaw",
    version,
    about = "Modular AI assistant for communities"
)]
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
    Dev {
        /// Watch for file changes and auto-reload
        #[arg(long)]
        watch: bool,
    },
    /// Validate config file
    Check,
    /// Manage skills (scaffold, test)
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },
    /// Open interactive web playground
    Playground {
        /// Port for playground server
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },
}

#[derive(Subcommand, Debug)]
pub enum SkillAction {
    /// Create a new skill from template
    New {
        /// Skill name (e.g. "my-skill")
        name: String,

        /// Language: "rust" or "python"
        #[arg(short, long, default_value = "rust")]
        lang: String,

        /// Output directory (defaults to current directory)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Run tests for a skill
    Test {
        /// Skill name or directory
        name: String,
    },
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
        assert!(matches!(cli.command, Some(Command::Dev { watch: false })));
    }

    #[test]
    fn test_cli_dev_watch() {
        let cli = Cli::parse_from(["amanclaw", "dev", "--watch"]);
        assert!(matches!(cli.command, Some(Command::Dev { watch: true })));
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

    #[test]
    fn test_cli_skill_new_rust_default() {
        let cli = Cli::parse_from(["amanclaw", "skill", "new", "my-skill"]);
        match cli.command {
            Some(Command::Skill {
                action: SkillAction::New { name, lang, output },
            }) => {
                assert_eq!(name, "my-skill");
                assert_eq!(lang, "rust");
                assert!(output.is_none());
            }
            _ => panic!("expected Skill New command"),
        }
    }

    #[test]
    fn test_cli_skill_new_python() {
        let cli = Cli::parse_from(["amanclaw", "skill", "new", "my-skill", "--lang", "python"]);
        match cli.command {
            Some(Command::Skill {
                action: SkillAction::New { name, lang, output },
            }) => {
                assert_eq!(name, "my-skill");
                assert_eq!(lang, "python");
                assert!(output.is_none());
            }
            _ => panic!("expected Skill New command"),
        }
    }

    #[test]
    fn test_cli_skill_new_with_output() {
        let cli = Cli::parse_from([
            "amanclaw",
            "skill",
            "new",
            "my-skill",
            "--output",
            "/tmp/skills",
        ]);
        match cli.command {
            Some(Command::Skill {
                action: SkillAction::New { output, .. },
            }) => {
                assert_eq!(output.as_deref(), Some("/tmp/skills"));
            }
            _ => panic!("expected Skill New command"),
        }
    }

    #[test]
    fn test_cli_playground() {
        let cli = Cli::parse_from(["amanclaw", "playground", "--port", "4000"]);
        assert!(matches!(
            cli.command,
            Some(Command::Playground { port: 4000 })
        ));
    }

    #[test]
    fn test_cli_playground_default_port() {
        let cli = Cli::parse_from(["amanclaw", "playground"]);
        assert!(matches!(
            cli.command,
            Some(Command::Playground { port: 3000 })
        ));
    }

    #[test]
    fn test_cli_skill_test() {
        let cli = Cli::parse_from(["amanclaw", "skill", "test", "my-skill"]);
        match cli.command {
            Some(Command::Skill {
                action: SkillAction::Test { name },
            }) => {
                assert_eq!(name, "my-skill");
            }
            _ => panic!("expected Skill Test command"),
        }
    }
}
