use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod config;
mod figma;

#[derive(Parser)]
#[command(name = "treble")]
#[command(about = "Figma-to-code CLI — sync designs to disk, explore layers, render nodes")]
#[command(version)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Store your Figma token (or authenticate via treble.build)
    Login {
        /// Use manual PAT entry instead of web-based login
        #[arg(long)]
        pat: bool,

        /// Pass Figma token directly (non-interactive, for scripts/agents)
        #[arg(long)]
        figma_token: Option<String>,

        /// Base URL for treble.build (default: https://treble.build)
        #[arg(long, default_value = "https://treble.build")]
        server: String,
    },

    /// Scaffold .treble/ in the current project directory
    Init {
        /// Figma file URL or key
        #[arg(long)]
        figma: Option<String>,

        /// Flavor: react-shadcn, wordpress-basecoat
        #[arg(long, default_value = "react-shadcn")]
        flavor: String,
    },

    /// Sync Figma data to .treble/figma/ (deterministic, git-friendly)
    Sync {
        /// Only sync frames matching this name (substring match)
        #[arg(long)]
        frame: Option<String>,

        /// Only sync frames from this page (substring match)
        #[arg(long)]
        page: Option<String>,

        /// Only sync a specific node ID or Figma URL (e.g. "254:1863" or a "Copy link to selection" URL)
        #[arg(long)]
        node: Option<String>,

        /// Re-sync all frames even if already synced
        #[arg(long)]
        force: bool,

        /// Interactively browse pages and pick frames to sync
        #[arg(long, short)]
        interactive: bool,
    },

    /// Show the layer tree for a frame (reads from synced data on disk)
    Tree {
        /// Frame name (required)
        frame: String,

        /// Max depth to display (default: unlimited)
        #[arg(long, short)]
        depth: Option<u32>,

        /// Show visual properties (fills, typography, layout)
        #[arg(long, short)]
        verbose: bool,

        /// Show only the subtree rooted at this node (name or ID, e.g. "NavBar" or "55:1234")
        #[arg(long)]
        root: Option<String>,

        /// Output as JSON instead of colored text (for agent consumption)
        #[arg(long)]
        json: bool,
    },

    /// Extract source images from Figma IMAGE fills to .treble/figma/{slug}/assets/
    Extract {
        /// Only extract images from frames matching this name (substring match)
        #[arg(long)]
        frame: Option<String>,
    },

    /// Render a specific Figma node and save the screenshot to disk
    Show {
        /// Node name or ID (e.g. "NavBar" or "55:1234")
        node: String,

        /// Frame to search in (required if using name instead of ID)
        #[arg(long)]
        frame: Option<String>,

        /// Render scale (default: 2)
        #[arg(long, default_value = "2")]
        scale: f64,

        /// Output as JSON (for agent consumption — includes saved path)
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Login {
            pat,
            figma_token,
            server,
        } => commands::login::run(pat, figma_token, server).await,
        Commands::Init { figma, flavor } => commands::init::run(figma, flavor).await,
        Commands::Sync {
            frame,
            page,
            node,
            force,
            interactive,
        } => commands::sync::run(frame, page, node, force, interactive).await,
        Commands::Extract { frame } => commands::extract::run(frame).await,
        Commands::Tree {
            frame,
            depth,
            verbose,
            root,
            json,
        } => commands::tree::run(frame, depth, verbose, root, json),
        Commands::Show {
            node,
            frame,
            scale,
            json,
        } => commands::show::run(node, frame, scale, json).await,
    }
}
