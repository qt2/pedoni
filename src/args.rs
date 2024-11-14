use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, clap::Parser, Deserialize)]
pub struct Args {
    /// Path to config file
    #[arg(short, long)]
    pub config: Option<PathBuf>,
    /// Runs in headless mode
    #[arg(short = 'H')]
    pub headless: bool,
}
