mod osm;

use std::any::Any;

use glam::Vec2;
use ndarray::Array2;
use thin_vec::ThinVec;

use super::{field::Field, scenario::Scenario, util::Index, Simulator};

pub use osm::OptimalStepsModel;

pub trait PedestrianModel {
    fn spawn_pedestrians(&mut self, pedestrians: Vec<Pedestrian>);

    fn calc_next_state(&self, sim: &Simulator) -> Box<dyn Any>;

    fn apply_next_state(&mut self, next_state: Box<dyn Any>);

    fn list_pedestrians(&self) -> Vec<Pedestrian>;

    fn get_pedestrian_count(&self) -> i32;
}

// pub struct Environment<'a> {
//     pub scenario: &'a Scenario,
//     pub field: &'a Field,
//     pub neighbor_grid: &'a Option<Array2<ThinVec<u32>>>,
//     pub neighbor_grid_belong: &'a Option<Vec<Index>>,
// }

/// Pedestrian instance
#[derive(Debug, Clone)]
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
