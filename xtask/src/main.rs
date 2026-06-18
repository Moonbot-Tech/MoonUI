mod transform;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Build automation for MoonUI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Transform Zed's GPUI crates into Moon standalone crates.
    Transform {
        /// Zed git tag to transform (e.g., v0.185.0)
        #[arg(long)]
        zed_tag: String,

        /// Path to local zed repo (optional, will clone if not provided)
        #[arg(long)]
        zed_path: Option<String>,

        /// Output directory for transformed crates (default: ./crates)
        #[arg(long, default_value = "crates")]
        output: String,

        /// Use path dependencies for local testing (instead of version deps)
        #[arg(long)]
        local: bool,
    },

    /// List crates in extraction order.
    ListCrates,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Transform {
            zed_tag,
            zed_path,
            output,
            local,
        } => transform::run(&zed_tag, zed_path.as_deref(), &output, local),
        Commands::ListCrates => {
            for crate_name in transform::CRATE_PUBLISH_ORDER {
                println!("{crate_name}");
            }
            Ok(())
        }
    }
}
