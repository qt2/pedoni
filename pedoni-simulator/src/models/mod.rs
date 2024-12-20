// mod osm;
// mod osm_gpu;
mod sfm;
mod sfm_gpu;

use glam::Vec2;

use crate::SimulatorOptions;

use super::{field::Field, scenario::Scenario, Simulator};
// use crate::args::Args;

#[allow(unused)]
pub use self::{
    // osm::OptimalStepsModel, osm_gpu::OptimalStepsModelGpu,
    sfm::SocialForceModel,
    sfm_gpu::SocialForceModelGpu,
};

pub trait PedestrianModel: Send + Sync {
    fn new(options: &SimulatorOptions, _scenario: &Scenario, _field: &Field) -> Self
    where
        Self: Sized;

    fn spawn_pedestrians(&mut self, new_pedestrians: Vec<Pedestrian>);

    fn calc_next_state(&self, sim: &Simulator);

    fn apply_next_state(&mut self);

    fn list_pedestrians(&self) -> Vec<Pedestrian>;

    fn get_pedestrian_count(&self) -> i32;
}

pub struct EmptyModel;

impl PedestrianModel for EmptyModel {
    fn new(_options: &SimulatorOptions, _scenario: &Scenario, _field: &Field) -> Self {
        todo!()
    }

    fn spawn_pedestrians(&mut self, _pedestrians: Vec<Pedestrian>) {
        todo!()
    }

    fn calc_next_state(&self, _sim: &Simulator) {
        todo!()
    }

    fn apply_next_state(&mut self) {
        todo!()
    }

    fn list_pedestrians(&self) -> Vec<Pedestrian> {
        todo!()
    }

    fn get_pedestrian_count(&self) -> i32 {
        todo!()
    }
}

/// Pedestrian instance
#[derive(Debug, Clone)]
pub struct Pedestrian {
    pub active: bool,
    pub pos: Vec2,
    pub destination: usize,
}

impl Default for Pedestrian {
    fn default() -> Self {
        Pedestrian {
            active: true,
            pos: Vec2::default(),
            destination: 0,
        }
    }
}
