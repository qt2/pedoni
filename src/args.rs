use std::path::PathBuf;

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum ModelType {
    Sfm,
    Osm,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum Backend {
    Cpu,
    Gpu,
}

#[derive(Debug, clap::Parser)]
pub struct Args {
    /// Path to scenario file
    #[arg(default_value = "scenarios/default.toml")]
    pub scenario: PathBuf,
    /// Runs in headless mode
    #[arg(short = 'H', long)]
    pub headless: bool,
    /// Model type
    #[arg(value_enum, short, long, default_value_t=ModelType::Osm)]
    pub model: ModelType,
    /// Backend
    #[arg(value_enum, short, long, default_value_t=Backend::Cpu)]
    pub backend: Backend,
    /// Do not use grid for acceleration
    #[arg(long)]
    pub no_grid: bool,
    /// Max playback speed
    #[arg(short, long, default_value_t = 100.0)]
    pub speed: f32,
    /// Unit length of field navigation grid
    #[arg(long)]
    pub field_unit: Option<f32>,
    /// Unit length of neighbor grid
    #[arg(long)]
    pub neighbor_unit: Option<f32>,
    /// Local work size of GPU kernel
    #[arg(long)]
    pub work_size: Option<usize>,
}
