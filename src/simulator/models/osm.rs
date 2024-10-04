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

    fn list_pedestrians(&self) -> Vec<f32> {
        // let state =
        todo!()
    }

    fn calc_next_state(&self) -> Box<dyn std::any::Any> {
        let state = vec![(Vec2::ZERO, false)];
        Box::new(state)
    }

    fn apply_next_state(&mut self, next_state: Box<dyn std::any::Any>) {
        let next_state = *next_state.downcast::<Vec<(Vec2, bool)>>().unwrap();
    }
}
