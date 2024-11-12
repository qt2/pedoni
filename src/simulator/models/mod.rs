use std::any::Any;

use glam::Vec2;

use super::field::Field;

mod osm;

pub trait PedestrianModel {
    fn spawn_pedestrians(&mut self);

    fn calc_next_state(&self, env: Environment) -> Box<dyn Any>;

    fn apply_next_state(&mut self, next_state: Box<dyn Any>);

    fn list_pedestrians(&self) -> Vec<f32>;
}

pub struct Environment<'a> {
    field: &'a Field,
}

/// Pedestrian instance
#[derive(Debug)]
pub struct Pedestrian {
    pub active: bool,
    pub pos: Vec2,
    pub destination: usize,
}

impl Default for Pedestrian {
    fn default() -> Self {
        // default parameters from https://arxiv.org/abs/cond-mat/9805244

        // let v0 = fastrand_contrib::f32_normal_approx(1.34, 0.26);

        Pedestrian {
            active: true,
            pos: Vec2::default(),
            destination: 0,
        }
    }
}
