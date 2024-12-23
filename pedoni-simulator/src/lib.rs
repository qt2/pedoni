pub mod diagnostic;
pub mod field;
mod models;
mod neighbor_grid;
pub mod scenario;
pub mod util;

use std::{sync::Mutex, time::Instant};

use diagnostic::{DiagnositcLog, StepMetrics};
use field::Field;
use log::info;
use models::{EmptyModel, Pedestrian, PedestrianModel, SocialForceModel, SocialForceModelGpu};
use scenario::Scenario;

/// Simulator instance.
pub struct Simulator {
    pub scenario: Scenario,
    pub field: Field,
    pub model: Box<dyn PedestrianModel>,
    pub spawn_rng: fastrand::Rng,
    pub diagnostic_log: DiagnositcLog,
    pub step_metrics: Mutex<StepMetrics>,
}

impl Simulator {
    pub fn new() -> Self {
        Simulator {
            scenario: Scenario::default(),
            field: Field::default(),
            model: Box::new(EmptyModel),
            spawn_rng: fastrand::Rng::new(),
            diagnostic_log: DiagnositcLog::default(),
            step_metrics: Mutex::new(StepMetrics::default()),
        }
    }

    pub fn initialize(&mut self, scenario: Scenario, options: &SimulatorOptions) {
        let field = Field::from_scenario(&scenario, options.field_grid_unit);
        let model: Box<dyn PedestrianModel> = match options.backend {
            Backend::Cpu => Box::new(SocialForceModel::new(options, &scenario, &field)),
            Backend::Gpu => Box::new(SocialForceModelGpu::new(options, &scenario, &field)),
        };

        self.scenario = scenario;
        self.field = field;
        self.model = model;
        self.spawn_rng = fastrand::Rng::with_seed(0);

        info!("Simulator initialization finished");
        info!("Simulator options: {options:#?}");
    }

    pub fn spawn_pedestrians(&mut self) {
        let instant = Instant::now();

        let mut new_pedestrians = Vec::new();
        for pedestrian in self.scenario.pedestrians.iter() {
            let [p_1, p_2] = self.scenario.waypoints[pedestrian.origin].line;
            let count = util::poisson(pedestrian.spawn.frequency / 10.0, &mut self.spawn_rng);

            for _ in 0..count {
                let pos = p_1.lerp(p_2, self.spawn_rng.f32());
                new_pedestrians.push(Pedestrian {
                    pos,
                    destination: pedestrian.destination,
                    ..Default::default()
                })
            }
        }
        self.model.spawn_pedestrians(&self.field, new_pedestrians);

        self.step_metrics.lock().unwrap().time_spawn = instant.elapsed().as_secs_f64();
    }

    pub fn calc_next_state(&self) {
        let instant = Instant::now();

        self.model.calc_next_state(self);

        self.step_metrics.lock().unwrap().time_calc_state = instant.elapsed().as_secs_f64();
    }

    pub fn apply_next_state(&mut self) {
        let instant = Instant::now();

        self.model.apply_next_state();

        self.step_metrics.lock().unwrap().time_apply_state = instant.elapsed().as_secs_f64();
    }

    pub fn collect_diagnostic_metrics(&mut self) {
        let mut metrics = {
            let mut metrics = self.step_metrics.lock().unwrap();
            let mut empty = StepMetrics::default();
            std::mem::swap(&mut *metrics, &mut empty);
            empty
        };
        metrics.active_ped_count = self.get_pedestrian_count();
        self.diagnostic_log.push(metrics);
    }

    pub fn list_pedestrians(&self) -> Vec<Pedestrian> {
        self.model.list_pedestrians()
    }

    pub fn get_pedestrian_count(&self) -> i32 {
        self.model.get_pedestrian_count()
    }
}

/// Simulator options.
#[derive(Debug)]
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
