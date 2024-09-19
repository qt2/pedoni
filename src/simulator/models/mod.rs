mod osm;

pub trait PedestrianModel {
    fn spawn_pedestrians(&mut self);

    fn spawn_obstacles(&mut self);

    fn tick(&mut self);

    fn list_pedestrians(&self) -> Vec<f32>;
}
