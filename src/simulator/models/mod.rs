mod osm;
mod osm_gpu;

use std::any::Any;

use glam::Vec2;

use super::Simulator;

pub use self::{osm::OptimalStepsModel, osm_gpu::OptimalStepsModelGpu};

pub trait PedestrianModel {
    fn spawn_pedestrians(&mut self, pedestrians: Vec<Pedestrian>);

    fn calc_next_state(&self, sim: &Simulator) -> Box<dyn Any + Send + Sync>;

    fn apply_next_state(&mut self, next_state: Box<dyn Any + Send + Sync>);

    fn list_pedestrians(&self) -> Vec<Pedestrian>;

    fn get_pedestrian_count(&self) -> i32;
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
