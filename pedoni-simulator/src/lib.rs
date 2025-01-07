pub mod diagnostic;
pub mod field;
pub mod models;
mod neighbor_grid;
pub mod scenario;
pub mod util;

use std::time::Instant;

use diagnostic::StepMetrics;
use field::Field;
use log::info;
use models::{Pedestrian, PedestrianModel, SocialForceModel, SocialForceModelGpu};
use scenario::{PedestrianSpawnConfig, Scenario};

/// Simulator instance.
pub struct Simulator {
    pub options: SimulatorOptions,
    pub scenario: Scenario,
    pub field: Field,
    pub model: Box<dyn PedestrianModel>,
    pub step: i32,
}

impl Simulator {
    // Prepare a new simulator with given options and scenario.
    pub fn new(options: SimulatorOptions, scenario: Scenario) -> Self {
        info!("Simulator options: {options:#?}");

        let field = Field::from_scenario(&scenario, options.field_grid_unit);

        let mut model: Box<dyn PedestrianModel> = match options.backend {
            Backend::Cpu => Box::new(SocialForceModel::new(&options, &scenario, &field)),
            Backend::Gpu => Box::new(SocialForceModelGpu::new(&options, &scenario, &field)),
        };

        let mut new_pedestrians = Vec::new();
        for pedestrian in scenario.pedestrians.iter() {
            if let PedestrianSpawnConfig::Once { count } = pedestrian.spawn {
                let [p_1, p_2] = scenario.waypoints[pedestrian.origin].line;

                for _ in 0..count {
                    let pos = p_1.lerp(p_2, fastrand::f32());
                    new_pedestrians.push(Pedestrian {
                        pos,
                        destination: pedestrian.destination,
                        ..Default::default()
                    })
                }
            }
        }
        model.spawn_pedestrians(&field, new_pedestrians);

        Simulator {
            options,
            scenario,
            field,
            model,
            step: 0,
        }
    }

    // Step the time and update pedestrians' positions.
    pub fn tick(&mut self) -> StepMetrics {
        self.step += 1;

        // Spawn / despawn pedestrians
        let instant = Instant::now();
        let mut new_pedestrians = Vec::new();
        for pedestrian in self.scenario.pedestrians.iter() {
            if let PedestrianSpawnConfig::Periodic { frequency } = pedestrian.spawn {
                let [p_1, p_2] = self.scenario.waypoints[pedestrian.origin].line;
                let count = util::poisson(frequency / 10.0);

                for _ in 0..count {
                    let pos = p_1.lerp(p_2, fastrand::f32());
                    new_pedestrians.push(Pedestrian {
                        pos,
                        destination: pedestrian.destination,
                        ..Default::default()
                    })
                }
            }
        }
        self.model.spawn_pedestrians(&self.field, new_pedestrians);
        let time_spawn = instant.elapsed().as_secs_f64();

        // Update states
        let instant = Instant::now();
        self.model.update_states(&self.scenario, &self.field);
        let time_calc_state = instant.elapsed().as_secs_f64();

        // Record performance metrics
        StepMetrics {
            active_ped_count: self.model.get_pedestrian_count(),
            time_spawn,
            time_calc_state,
            time_calc_state_kernel: None,
        }
    }

    pub fn list_pedestrians(&self) -> Vec<Pedestrian> {
        self.model.list_pedestrians()
    }
}

/// Simulator options.
#[derive(Debug, Clone)]
pub struct SimulatorOptions {
    /// Backend type: CPU or GPU    
    pub backend: Backend,
    /// Unit length of the neighbor search grid. (meters)
    pub neighbor_grid_unit: f32,
    /// Unit length of potential maps and distance maps. (meters)
    pub field_grid_unit: f32,
    /// Whether to use neighbor search grid.
    pub use_neighbor_grid: bool,
    /// Whether to use a descretized distance map for calculating repusive effects against obstacles.
    pub use_distance_map: bool,
    /// Local workgroup size of GPU kernels.
    pub gpu_work_size: usize,
}

impl Default for SimulatorOptions {
    fn default() -> Self {
        SimulatorOptions {
            backend: Backend::Cpu,
            neighbor_grid_unit: 1.4,
            field_grid_unit: 0.25,
            use_neighbor_grid: true,
            use_distance_map: true,
            gpu_work_size: 64,
        }
    }
}

/// Simulator backend.
#[derive(Debug, Clone, Copy)]
pub enum Backend {
    Cpu,
    Gpu,
}
