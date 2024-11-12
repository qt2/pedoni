pub mod diagnostic;
pub mod field;
mod models;
pub mod optim;
pub mod scenario;
pub mod util;

use std::{any::Any, sync::Arc};

use crate::{State, STATE};
use eframe::wgpu;
use field::Field;
use models::{OptimalStepsModel, OptimalStepsModelGpu, Pedestrian, PedestrianModel};
use ndarray::Array2;

use scenario::Scenario;
use thin_vec::ThinVec;
use util::Index;

/// Simulator instance
pub struct Simulator {
    pub scenario: Scenario,
    pub field: Field,
    pub model: OptimalStepsModelGpu,
    pub neighbor_grid: Option<Array2<ThinVec<u32>>>,
    pub neighbor_grid_belong: Option<Vec<Index>>,
}

impl Simulator {
    pub fn new() -> Self {
        Simulator {
            scenario: Scenario::default(),
            field: Field::default(),
            model: OptimalStepsModelGpu::new(),
            neighbor_grid: None,
            neighbor_grid_belong: None,
        }
    }

    /// Create new simulator instance with scenario
    pub fn load_scenario(&mut self, scenario: Scenario) {
        let field = Field::from_scenario(&scenario);

        self.scenario = scenario;
        self.field = field;
        // self.model = OptimalStepsModelGpu::new();
        self.neighbor_grid = None;
        self.neighbor_grid_belong = None;
    }

    pub fn spawn_pedestrians(&mut self) {
        let mut new_pedestrians = Vec::new();

        for pedestrian in self.scenario.pedestrians.iter() {
            let [p_1, p_2] = self.scenario.waypoints[pedestrian.origin].line;
            let count = util::poisson(pedestrian.spawn.frequency / 10.0);

            for _ in 0..count {
                let pos = p_1.lerp(p_2, fastrand::f32());
                new_pedestrians.push(Pedestrian {
                    pos,
                    destination: pedestrian.destination,
                    ..Default::default()
                })
            }
        }

        self.model.spawn_pedestrians(new_pedestrians);

        let pedestrians = self.model.list_pedestrians();

        let State {
            use_neighbor_grid,
            neighbor_grid_unit,
            ..
        } = *STATE.lock().unwrap();

        (self.neighbor_grid, self.neighbor_grid_belong) = if use_neighbor_grid {
            let shape = (self.scenario.field.size / neighbor_grid_unit).ceil();
            let shape = (shape.y as usize, shape.x as usize);
            let mut grid = Array2::from_elem(shape, ThinVec::new());
            let mut belong = vec![Index::default(); pedestrians.len()];

            for (i, pedestrian) in pedestrians.iter().enumerate().filter(|(_, ped)| ped.active) {
                let ix = (pedestrian.pos / neighbor_grid_unit).ceil().as_ivec2();
                let ix = Index::new(ix.x, ix.y);
                if let Some(neighbors) = grid.get_mut(ix) {
                    neighbors.push(i as u32);
                    belong[i] = ix;
                }
            }

            (Some(grid), Some(belong))
        } else {
            (None, None)
        };
    }

    pub fn calc_next_state(&self) -> Box<dyn Any> {
        self.model.calc_next_state(self)
    }

    pub fn apply_next_state(&mut self, state: Box<dyn Any>) {
        self.model.apply_next_state(state);
    }

    pub fn list_pedestrians(&self) -> Vec<Pedestrian> {
        self.model.list_pedestrians()
    }

    pub fn get_pedestrian_count(&self) -> i32 {
        self.model.get_pedestrian_count()
    }
}
