mod component_api;
mod component_audit;
mod component_mirror;
mod transform;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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

        /// Use version dependencies for publish-style generated crates.
        #[arg(long)]
        versioned_deps: bool,
    },

    /// List crates in extraction order.
    ListCrates,

    /// Audit MoonUI component architecture and compare against a baseline.
    ComponentAudit {
        /// Baseline JSON path.
        #[arg(long, default_value = "docs/component-audit-baseline.json")]
        baseline: PathBuf,

        /// Write the current audit report as the new baseline.
        #[arg(long)]
        update_baseline: bool,

        /// Compare the current audit report with the baseline.
        #[arg(long)]
        check_baseline: bool,

        /// Print the full audit report as JSON.
        #[arg(long)]
        json: bool,
    },

    /// Snapshot Moon-facing public component API and compare against a baseline.
    ComponentApi {
        /// Baseline JSON path.
        #[arg(long, default_value = "docs/component-api-baseline.json")]
        baseline: PathBuf,

        /// Write the current API snapshot as the new baseline.
        #[arg(long)]
        update_baseline: bool,

        /// Compare the current API snapshot with the baseline.
        #[arg(long)]
        check_baseline: bool,

        /// Print the full API snapshot as JSON.
        #[arg(long)]
        json: bool,
    },

    /// Track Mirror components against their Longbridge source paths.
    ComponentMirror {
        /// Baseline JSON path.
        #[arg(long, default_value = "docs/component-mirror-baseline.json")]
        baseline: PathBuf,

        /// Optional Longbridge donor src root, for example vendor/gpui-component/crates/ui/src.
        #[arg(long)]
        donor_root: Option<PathBuf>,

        /// Write the current mirror report as the new baseline.
        #[arg(long)]
        update_baseline: bool,

        /// Compare the current mirror report with the baseline.
        #[arg(long)]
        check_baseline: bool,

        /// Print the full mirror report as JSON.
        #[arg(long)]
        json: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Transform {
            zed_tag,
            zed_path,
            output,
            versioned_deps,
        } => transform::run(&zed_tag, zed_path.as_deref(), &output, !versioned_deps),
        Commands::ListCrates => {
            for crate_name in transform::CRATE_PUBLISH_ORDER {
                println!("{crate_name}");
            }
            Ok(())
        }
        Commands::ComponentAudit {
            baseline,
            update_baseline,
            check_baseline,
            json,
        } => component_audit::run(component_audit::AuditOptions {
            baseline,
            update_baseline,
            check_baseline,
            json,
        }),
        Commands::ComponentApi {
            baseline,
            update_baseline,
            check_baseline,
            json,
        } => component_api::run(component_api::ApiOptions {
            baseline,
            update_baseline,
            check_baseline,
            json,
        }),
        Commands::ComponentMirror {
            baseline,
            donor_root,
            update_baseline,
            check_baseline,
            json,
        } => component_mirror::run(component_mirror::MirrorOptions {
            baseline,
            donor_root,
            update_baseline,
            check_baseline,
            json,
        }),
    }
}
