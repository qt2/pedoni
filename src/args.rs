use std::path::PathBuf;

// use serde::Deserialize;

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum ModelType {
    #[value(name = "no_grid")]
    OsmNoGrid,
    #[value(name = "cpu")]
    OsmCpu,
    #[value(name = "gpu")]
    OsmGpu,
}

#[derive(Debug, clap::Parser)]
pub struct Args {
    /// Path to scenario file
    #[arg(default_value = "scenarios/default.toml")]
    pub scenario: PathBuf,
    /// Path to config file
    #[arg(short, long)]
    pub config: Option<PathBuf>,
    /// Runs in headless mode
    #[arg(short = 'H', long)]
    pub headless: bool,
    /// Model type
    #[arg(value_enum, short, long, default_value_t=ModelType::OsmCpu)]
    pub model: ModelType,
    /// Max playback speed
    #[arg(short, long, default_value_t = 100.0)]
    pub speed: f32,
}
