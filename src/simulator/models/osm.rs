use glam::Vec2;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

use super::{Environment, PedestrianModel};

pub struct OptimalStepsModel {
    pedestrians: Vec<super::Pedestrian>,
}

impl OptimalStepsModel {
    pub fn new() -> Self {
        OptimalStepsModel {
            pedestrians: Vec::new(),
        }
    }
}

impl PedestrianModel for OptimalStepsModel {
    fn spawn_pedestrians(&mut self) {}

    fn list_pedestrians(&self) -> Vec<f32> {
        // let state =
        todo!()
    }

    fn calc_next_state(&self, env: Environment) -> Box<dyn std::any::Any> {
        const R: f32 = 0.3;

        let state: Vec<_> = self
            .pedestrians
            .par_iter()
            .enumerate()
            .filter(|(_, ped)| ped.active)
            .map(|(i, ped)| {
                let active = env.field.get_potential(ped.destination, ped.pos) > 2.0;

                // const Q: i32 = 16;

                // let (potential, position) = (0..Q)
                //     .map(|k| {
                //         let phi = 2.0 * PI / Q as f32 * (k as f32 + fastrand::f32());
                //         let x_k = ped.pos + R * vec2(phi.cos(), phi.sin());

                //         let p = self.get_potential(i, ped.destination, x_k);

                //         (NotNan::new(p).unwrap(), x_k)
                //     })
                //     .min_by_key(|t| t.0)
                //     .unwrap();

                let f = |x: Vec2| self.get_potential(i, ped.destination, ped.pos + x);

                let position = util::nelder_mead(
                    f,
                    vec![Vec2::ZERO, vec2(0.05, 0.00025), vec2(0.00025, 0.05)],
                    Some(R),
                ) + ped.pos;

                (position, active)
            })
            .collect();
        Box::new(state)
    }

    fn apply_next_state(&mut self, next_state: Box<dyn std::any::Any>) {
        let next_state = *next_state.downcast::<Vec<(Vec2, bool)>>().unwrap();
    }
}
