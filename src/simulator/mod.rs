pub mod diagnostic;
pub mod field;
mod models;
mod neighbor_grid;
pub mod optim;
pub mod scenario;
pub mod util;

use std::{sync::Mutex, time::Instant};

use crate::args::{Args, ModelType};
use diagnostic::{DiagnositcLog, StepMetrics};
use field::Field;
use models::{EmptyModel, OptimalStepsModel, OptimalStepsModelGpu, Pedestrian, PedestrianModel};
pub use neighbor_grid::NeighborGrid;
use scenario::Scenario;

/// Simulator instance
pub struct Simulator {
    pub scenario: Scenario,
    pub field: Field,
    pub model: Box<dyn PedestrianModel>,
    pub spawn_rng: fastrand::Rng,
    pub neighbor_grid: Option<NeighborGrid>,

    pub diagnostic_log: DiagnositcLog,
    pub step_metrics: Mutex<StepMetrics>,
}

impl Simulator {
    pub fn empty() -> Self {
        Simulator {
            scenario: Scenario::default(),
            field: Field::default(),
            model: Box::new(EmptyModel),
            spawn_rng: fastrand::Rng::new(),
            neighbor_grid: None,

            diagnostic_log: DiagnositcLog::default(),
            step_metrics: Mutex::new(StepMetrics::default()),
        }
    }

    pub fn initialize(&mut self, scenario: Scenario, args: &Args) {
        let field = Field::from_scenario(&scenario);
        let model: Box<dyn PedestrianModel> = match args.model {
            ModelType::OptimalStepsModel => {
                Box::new(OptimalStepsModel::new(args, &scenario, &field))
            }
            ModelType::OptimalStepsModelGpu => {
                Box::new(OptimalStepsModelGpu::new(args, &scenario, &field))
            }
        };

        self.neighbor_grid = Some(NeighborGrid::new(scenario.field.size, 0.6));
        self.scenario = scenario;
        self.field = field;
        self.model = model;
        self.spawn_rng = fastrand::Rng::with_seed(0);
        self.diagnostic_log.model = format!("{:?}", args.model);
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

        self.model.spawn_pedestrians(new_pedestrians);

        if let Some(grid) = &mut self.neighbor_grid {
            grid.update(self.model.list_pedestrians().iter().map(|p| p.pos));
        }

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
