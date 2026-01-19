//! patto-gcal-sync: Synchronize Patto task deadlines to Google Calendar
//!
//! Usage:
//!   patto-gcal-sync auth         # Authenticate with Google Calendar
//!   patto-gcal-sync sync         # Sync tasks to calendar
//!   patto-gcal-sync sync --dry-run  # Preview sync actions
//!   patto-gcal-sync revoke       # Revoke credentials

use anyhow::Result;
use clap::{Parser, Subcommand};
use patto::gcal::{auth, GcalConfig, GcalSync};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "patto-gcal-sync")]
#[command(about = "Synchronize Patto task deadlines to Google Calendar")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to patto repository (defaults to current directory)
    #[arg(short, long, global = true)]
    path: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with Google Calendar (performs OAuth flow)
    Auth,

    /// Synchronize tasks to Google Calendar
    Sync {
        /// Preview changes without applying them
        #[arg(long)]
        dry_run: bool,
    },

    /// Revoke stored credentials
    Revoke,

    /// Show current configuration
    Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose {
        simplelog::LevelFilter::Debug
    } else {
        simplelog::LevelFilter::Info
    };
    simplelog::TermLogger::init(
        log_level,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )?;

    // Determine repository path
    let repo_path = cli.path.unwrap_or_else(|| std::env::current_dir().unwrap());

    match cli.command {
        Commands::Auth => {
            println!("🔐 Starting Google Calendar authentication...");
            println!();
            println!("This will open a browser window for you to authorize patto.");
            println!("Make sure you have configured client_id and client_secret in:");
            println!("  ~/.config/patto/patto-lsp.toml");
            println!();

            let config = GcalConfig::load()?;
            
            if config.client_id.is_none() || config.client_secret.is_none() {
                println!("❌ Missing OAuth credentials!");
                println!();
                println!("Please add the following to ~/.config/patto/patto-lsp.toml:");
                println!();
                println!("[google_calendar]");
                println!("client_id = \"your-client-id.apps.googleusercontent.com\"");
                println!("client_secret = \"your-client-secret\"");
                println!();
                println!("To obtain credentials:");
                println!("1. Go to https://console.cloud.google.com/");
                println!("2. Create a new project (or select existing)");
                println!("3. Enable the Google Calendar API");
                println!("4. Go to Credentials → Create Credentials → OAuth 2.0 Client ID");
                println!("5. Choose 'Desktop app' as application type");
                println!("6. Copy the client ID and client secret to your config");
                return Ok(());
            }

            let _auth = auth::authenticate_interactive(&config).await?;
            println!();
            println!("✅ Authentication successful!");
            println!("Credentials saved to: {:?}", GcalConfig::credentials_path()?);
        }

        Commands::Sync { dry_run } => {
            if dry_run {
                println!("🔍 Dry run mode - no changes will be made");
                println!();
            }

            let config = GcalConfig::load()?;
            
            if !auth::credentials_exist() {
                println!("❌ Not authenticated!");
                println!("Please run: patto-gcal-sync auth");
                return Ok(());
            }

            println!("📅 Syncing tasks to Google Calendar...");
            println!("Repository: {}", repo_path.display());
            println!();

            let mut sync = GcalSync::new(config).await?;
            let stats = sync.sync(&repo_path, dry_run).await?;

            println!();
            if dry_run {
                println!("📊 Would make the following changes:");
            } else {
                println!("📊 Sync complete:");
            }
            println!("  ✅ Created: {}", stats.created);
            println!("  🔄 Updated: {}", stats.updated);
            println!("  🗑️  Deleted: {}", stats.deleted);
            println!("  ⏭️  Skipped: {}", stats.skipped);

            if !stats.errors.is_empty() {
                println!();
                println!("⚠️  Errors ({}):", stats.errors.len());
                for error in &stats.errors {
                    println!("  - {}", error);
                }
            }
        }

        Commands::Revoke => {
            println!("🔓 Revoking credentials...");
            auth::revoke_credentials().await?;
            println!("✅ Credentials revoked successfully");
        }

        Commands::Config => {
            let config = GcalConfig::load()?;
            println!("📋 Current configuration:");
            println!();
            println!("Config file: {:?}", GcalConfig::config_file_path()?);
            println!("Credentials: {:?}", GcalConfig::credentials_path()?);
            println!("State file:  {:?}", GcalConfig::state_path()?);
            println!();
            println!("[google_calendar]");
            println!("calendar_id = \"{}\"", config.calendar_id);
            println!("event_prefix = \"{}\"", config.event_prefix);
            println!("sync_done_tasks = {}", config.sync_done_tasks);
            println!("include_file_path = {}", config.include_file_path);
            println!(
                "timezone = {:?}",
                config.timezone.as_deref().unwrap_or("(system default)")
            );
            println!("default_duration_hours = {}", config.default_duration_hours);
            println!(
                "client_id = {}",
                if config.client_id.is_some() {
                    "(configured)"
                } else {
                    "(not set)"
                }
            );
            println!(
                "client_secret = {}",
                if config.client_secret.is_some() {
                    "(configured)"
                } else {
                    "(not set)"
                }
            );
        }
    }

    Ok(())
}
