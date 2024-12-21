use std::path::PathBuf;

use pedoni_simulator::SimulatorOptions;

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
    /// Backend
    #[arg(value_enum, short, long, default_value_t=Backend::Cpu)]
    pub backend: Backend,
    /// Max playback speed
    #[arg(short, long, default_value_t = 100.0)]
    pub speed: f32,

    /// Do not use grid for acceleration
    #[arg(long)]
    pub no_neighbor_grid: bool,
    /// Do not use distance map
    #[arg(long)]
    pub no_distance_map: bool,
    /// Unit length of field navigation grid
    #[arg(long)]
    pub field_unit: Option<f32>,
    /// Unit length of neighbor search grid
    #[arg(long)]
    pub neighbor_unit: Option<f32>,
    /// Local work size of GPU kernel
    #[arg(long)]
    pub work_size: Option<usize>,
}

impl Args {
    pub fn to_simulator_options(&self) -> SimulatorOptions {
        let mut options = SimulatorOptions {
            backend: match self.backend {
                Backend::Cpu => pedoni_simulator::Backend::Cpu,
                Backend::Gpu => pedoni_simulator::Backend::Gpu,
            },
            use_neighbor_grid: !self.no_neighbor_grid,
            use_distance_map: !self.no_distance_map,
            ..Default::default()
        };

        if let Some(field_unit) = self.field_unit {
            options.field_grid_unit = field_unit;
        }
        if let Some(neighbor_unit) = self.neighbor_unit {
            options.neighbor_grid_unit = neighbor_unit;
        }

        options
    }
}
