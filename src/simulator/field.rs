use core::f32;
use std::{cmp::Reverse, collections::BinaryHeap};

use geo::LineString;
use geo_rasterize::{BinaryBuilder, LabelBuilder};
use glam::{vec2, IVec2, Mat3, Mat3A, Vec2, Vec3A};
use ndarray::{s, Array2};
use ordered_float::NotNan;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};

use super::{
    scenario::{ObstacleConfig, Scenario, WaypointConfig},
    util::{self, Index},
};

pub struct FieldBuilder {
    unit: f32,
    shape: (usize, usize),
    obstacle_exist: Array2<bool>,
    potentials: Vec<Array2<f32>>,
}

impl FieldBuilder {
    pub fn new(size: Vec2, unit: f32) -> Self {
        let grid_size = (size / unit).ceil();
        let shape = (grid_size.y as usize, grid_size.x as usize);
        let mut obstacle_exist = Array2::from_elem(shape, false);

        obstacle_exist.slice_mut(s![0, ..]).fill(true);
        obstacle_exist.slice_mut(s![-1, ..]).fill(true);
        obstacle_exist.slice_mut(s![.., 0]).fill(true);
        obstacle_exist.slice_mut(s![.., -1]).fill(true);

        FieldBuilder {
            unit,
            shape,
            obstacle_exist,
            potentials: Vec::new(),
        }
    }

    fn add_obstacle(&mut self, obstacle: &ObstacleConfig) {
        let vertices = util::line_with_width(obstacle.line, obstacle.width);
        let mut shape = LineString::from(
            vertices
                .into_iter()
                .map(|v| {
                    let v = v / self.unit;
                    (v.x, v.y)
                })
                .collect::<Vec<_>>(),
        );
        shape.close();

        let mut rasterizer = BinaryBuilder::new()
            .width(self.shape.1)
            .height(self.shape.0)
            .build()
            .unwrap();
        rasterizer.rasterize(&shape).unwrap();
        let grid = rasterizer.finish();

        self.obstacle_exist.zip_mut_with(&grid, |a, b| *a |= b);
    }

    fn add_waypoint(&mut self, waypoint: &WaypointConfig) {
        let vertices = util::line_with_width(waypoint.line, waypoint.width);
        let mut shape = LineString::from(
            vertices
                .into_iter()
                .map(|v| {
                    let v = v / self.unit;
                    (v.x, v.y)
                })
                .collect::<Vec<_>>(),
        );
        shape.close();

        let mut rasterizer = LabelBuilder::background(f32::MAX)
            .width(self.shape.1)
            .height(self.shape.0)
            .build()
            .unwrap();
        rasterizer.rasterize(&shape, 0.0).unwrap();
        let grid = rasterizer.finish();

        self.potentials.push(grid);
    }

    fn build(self) -> Field {
        let FieldBuilder {
            unit,
            shape,
            obstacle_exist,
            mut potentials,
        } = self;

        let mut distance_from_obstacle = obstacle_exist.map(|&obs| if obs { 0.0 } else { 1e24 });
        apply_fmm(&mut distance_from_obstacle, &Array2::from_elem(shape, unit));

        let slowness = distance_from_obstacle.map(|&d| (1e4 * (-10.0 * d).exp() + 1.0) * unit);
        potentials.par_iter_mut().for_each(|potential| {
            apply_fmm(potential, &slowness);
        });

        Field {
            unit,
            shape,
            obstacle_exist,
            distance_from_obstacle,
            potentials,
        }
    }
}

/// Calculate potential against a waypoint using Sethian's [fast marching method](https://en.wikipedia.org/wiki/Fast_marching_method).    
fn apply_fmm(potentials: &mut Array2<f32>, f: &Array2<f32>) {
    use Status::*;
    type Float = Reverse<NotNan<f32>>;

    #[derive(Debug, Clone, PartialEq, PartialOrd)]
    enum Status {
        Far,
        Considered,
        Accepted,
    }

    // const F_DEF: f32 = 1.0;
    // const F_OBS: f32 = 1e4;

    let shape = potentials.dim();
    let mut states = Array2::from_elem(shape, Status::Far);
    let mut queue = BinaryHeap::<(Float, Index)>::new();
    let float = |x: f32| Reverse(NotNan::new(x).unwrap());

    for y in 0..shape.0 {
        for x in 0..shape.1 {
            let ix = Index::new(x, y);
            if potentials[ix] == 0.0 {
                states[ix] = Accepted;

                for (j, i) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let ix = ix.add(i, j);
                    match states.get_mut(ix) {
                        None | Some(Accepted) => {}
                        Some(state) => {
                            if potentials[ix] == 0.0 {
                                continue;
                            }
                            *state = Considered;
                            let u = f[ix];
                            potentials[ix] = u;
                            queue.push((float(u), ix));
                        }
                    }
                }
            }
        }
    }

    while let Some((u, ix)) = queue.pop() {
        let u = u.0.into_inner();
        if states[ix] == Accepted {
            continue;
        }
        states[ix] = Accepted;

        for (j, i) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let ix = ix.add(i, j);

            match states.get_mut(ix) {
                None | Some(Accepted) => continue,
                Some(state) => *state = Considered,
            }

            let f = f[ix];
            let (u1, u2) = if j == 0 {
                let u2a = potentials.get(ix.add(0, -1)).cloned().unwrap_or(f32::MAX);
                let u2b = potentials.get(ix.add(0, 1)).cloned().unwrap_or(f32::MAX);
                (u, u2a.min(u2b))
            } else {
                let u1a = potentials.get(ix.add(-1, 0)).cloned().unwrap_or(f32::MAX);
                let u1b = potentials.get(ix.add(1, 0)).cloned().unwrap_or(f32::MAX);
                (u1a.min(u1b), u)
            };

            let u = if u1 == f32::MAX {
                u2 + f
            } else if u2 == f32::MAX {
                u1 + f
            } else {
                let sq = 2.0 * f * f - (u1 - u2).powi(2);
                if sq >= 0.0 {
                    (u1 + u2 + sq.sqrt()) / 2.0
                } else {
                    u1.min(u2) + f
                }
            };

            if u < potentials[ix] {
                potentials[ix] = u;
                queue.push((float(u), ix));
            }
        }
    }
}

pub struct Field {
    /// Unit of length (in meters)
    pub unit: f32,
    /// Shape of 2D grid (y, x)
    pub shape: (usize, usize),
    /// Boolean grid which holds obstacle existence
    pub obstacle_exist: Array2<bool>,
    /// Distance from nearest obstacle
    pub distance_from_obstacle: Array2<f32>,
    /// Potential against each waypoint
    pub potentials: Vec<Array2<f32>>,
}

impl Default for Field {
    fn default() -> Self {
        Field {
            unit: 0.5,
            shape: (0, 0),
            obstacle_exist: Default::default(),
            distance_from_obstacle: Default::default(),
            potentials: Vec::default(),
        }
    }
}

impl Field {
    pub fn from_scenario(scenario: &Scenario, unit: f32) -> Self {
        let mut builder = FieldBuilder::new(scenario.field.size, unit);

        for obstacle in scenario.obstacles.iter() {
            builder.add_obstacle(obstacle);
        }

        for waypoint in scenario.waypoints.iter() {
            builder.add_waypoint(waypoint);
        }

        builder.build()
    }

    pub fn get_field_potential(&self, waypoint_id: usize, position: Vec2) -> f32 {
        let position = position / self.unit - Vec2::splat(0.5);

        let potential = &self.potentials[waypoint_id];
        util::bilinear(potential, position)
    }

    pub fn get_distance_from_obstacles(&self, position: Vec2) -> f32 {
        let position = position / self.unit - Vec2::splat(0.5);
        util::bilinear(&self.distance_from_obstacle, position)
    }

    pub fn get_potential_grad(&self, waypoint_id: usize, position: Vec2) -> Vec2 {
        // const S_X: Mat3A = Mat3A::from_cols(
        //     Vec3A::new(1.0, 0.0, -1.0),
        //     Vec3A::new(2.0, 0.0, -2.0),
        //     Vec3A::new(1.0, 0.0, -1.0),
        // );
        // const S_Y: Mat3A = Mat3A::from_cols(
        //     Vec3A::new(1.0, 2.0, 1.0),
        //     Vec3A::new(0.0, 0.0, 0.0),
        //     Vec3A::new(-1.0, -2.0, -1.0),
        // );

        let potential = &self.potentials[waypoint_id];
        let position = position / self.unit - Vec2::splat(0.5);
        let mut grad = Vec2::ZERO;

        for y in -1..=1 {
            for x in [-1, 1] {
                let position = position + vec2(x as f32, y as f32);
                let u = util::bilinear(&potential, position);
                let s_x = (if y == 0 { 2.0 } else { 1.0 }) * -x as f32;
                grad.x += u * s_x;
            }
        }

        for x in -1..=1 {
            for y in [-1, 1] {
                let position = position + vec2(x as f32, y as f32);
                let u = util::bilinear(&potential, position);
                let s_y = (if x == 0 { 2.0 } else { 1.0 }) * -y as f32;
                grad.y += u * s_y;
            }
        }

        grad
    }
}

#[cfg(test)]
mod tests {
    use geo::{LineString, Polygon};
    use geo_rasterize::BinaryBuilder;
    use glam::vec2;
    use ndarray::Array2;

    use crate::simulator::scenario::{FieldConfig, ObstacleConfig, Scenario, WaypointConfig};

    use super::Field;

    #[test]
    fn test_obstacle() {
        let shape = Polygon::new(
            LineString::from(vec![(5.0, 3.5), (5.0, 4.5), (15.0, 4.5), (15.0, 3.5)]),
            vec![],
        );

        let mut rasterizer = BinaryBuilder::new().width(20).height(10).build().unwrap();
        rasterizer.rasterize(&shape).unwrap();
        let grid = rasterizer.finish();
        println!("{grid:#?}");

        println!("{:?}", Array2::<i32>::zeros((4, 2)));
    }

    #[test]
    fn test_parse_scenario() {
        let scenario = Scenario {
            field: FieldConfig {
                size: vec2(5.0, 5.0),
            },
            obstacles: vec![
                ObstacleConfig {
                    line: [vec2(0.0, 1.5), vec2(4.0, 1.5)],
                    ..Default::default()
                },
                ObstacleConfig {
                    line: [vec2(1.0, 3.5), vec2(5.0, 3.5)],
                    ..Default::default()
                },
            ],
            waypoints: vec![WaypointConfig {
                line: [vec2(0.0, 0.0), vec2(0.0, 1.0)],
                ..Default::default()
            }],
            ..Default::default()
        };

        let field = Field::from_scenario(&scenario, 0.25);

        println!("{:?}", field.obstacle_exist.map(|v| if *v { 1 } else { 0 }));

        println!("{:?}", field.potentials[0].map(|v| *v as i32));

        println!("{:?}", field.get_field_potential(0, vec2(-1.5, 2.0)));

        // let mut target_map = Array2::from_elem(field.shape, false);
        // target_map[(10, 5)] = true;
        // // target_map[(10, 6)] = true;
        // let potential = field.calc_potential(target_map);

        // println!("{:#?}", potential.map(|v| *v as i32));
    }
}
