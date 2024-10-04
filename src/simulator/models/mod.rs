use std::any::Any;

mod osm;

pub trait PedestrianModel {
    fn spawn_pedestrians(&mut self);

    fn calc_next_state(&self) -> Box<dyn Any>;

    fn apply_next_state(&mut self, next_state: Box<dyn Any>);

    fn list_pedestrians(&self) -> Vec<f32>;
}

struct Sim {
    model: Box<dyn PedestrianModel>,
}
