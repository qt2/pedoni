mod sfm;
// mod sfm_gpu;

use glam::Vec2;

use crate::map::MapData;

use super::{field::Field, scenario::Scenario};

#[allow(unused)]
pub use self::{
    sfm::SocialForceModel,
    //  sfm_gpu::SocialForceModelGpu
};

pub trait PedestrianModel: Send + Sync {
    fn load_map(&mut self, map: MapData);

    fn spawn_pedestrians(&mut self, new_pedestrians: Vec<Pedestrian>);

    fn update_states(&mut self);

    fn list_pedestrians(&self) -> Vec<Pedestrian>;

    fn pedestrian_count(&self) -> i32;
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
