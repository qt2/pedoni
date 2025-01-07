mod sfm;
mod sfm_gpu;

use glam::Vec2;

use crate::SimulatorOptions;

use super::{field::Field, scenario::Scenario, Simulator};

#[allow(unused)]
pub use self::{sfm::SocialForceModel, sfm_gpu::SocialForceModelGpu};

pub trait PedestrianModel: Send + Sync {
    fn new(options: &SimulatorOptions, _scenario: &Scenario, _field: &Field) -> Self
    where
        Self: Sized;

    fn spawn_pedestrians(&mut self, field: &Field, new_pedestrians: Vec<Pedestrian>);

    fn calc_next_state(&self, sim: &Simulator);

    fn apply_next_state(&mut self);

    fn list_pedestrians(&self) -> Vec<Pedestrian>;

    fn get_pedestrian_count(&self) -> i32;
}

/// Pedestrian instance
#[derive(Debug, Clone)]
pub struct Pedestrian {
    pub pos: Vec2,
    pub destination: usize,
}

impl Default for Pedestrian {
    fn default() -> Self {
        Pedestrian {
            pos: Vec2::default(),
            destination: 0,
        }
    }
}
