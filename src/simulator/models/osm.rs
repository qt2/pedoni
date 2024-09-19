use glam::Vec2;

use super::PedestrianModel;

pub struct OSM {
    p_positions: Vec<Vec2>,
    p_velocities: Vec<Vec2>,
}

impl PedestrianModel for OSM {
    fn spawn_pedestrians(&mut self) {
        todo!()
    }

    fn spawn_obstacles(&mut self) {
        todo!()
    }

    fn tick(&mut self) {
        todo!()
    }

    fn list_pedestrians(&self) -> Vec<f32> {
        todo!()
    }
}
