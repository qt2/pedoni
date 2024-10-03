use core::f32;
use std::{cmp::Reverse, collections::BinaryHeap};

use geo::Line;
use geo_rasterize::BinaryBuilder;
use glam::Vec2;
use ndarray::{s, Array2};
use num_traits::PrimInt;
use ordered_float::NotNan;
use thin_vec::ThinVec;

use super::scenario::{ObstacleConfig, Scenario, WaypointConfig};

pub struct Environment {
    /// Unit of length (in meters)
    pub unit: f32,
    /// Shape of 2D grid (y, x)
    pub shape: (usize, usize),
    /// Boolean grid which holds obstacle existence
    pub obstacle_existence: Array2<bool>,
    /// Vector grid indicating which obstacles each cell overlaps
    pub obstacle_map: Array2<ThinVec<u32>>,
    /// Potential against each waypoint
    pub potentials: Vec<Array2<f32>>,
}

impl Default for Environment {
    fn default() -> Self {
        Environment {
            unit: 0.5,
            shape: (0, 0),
            obstacle_existence: Default::default(),
            obstacle_map: Default::default(),
            potentials: Vec::default(),
        }
    }
}

impl Environment {
    pub fn from_scenario(scenario: &Scenario) -> Self {
        let unit = 0.25;
        let size = (scenario.field.size / unit).ceil().as_ivec2();
        let shape = (size.y as usize, size.x as usize);
        let obstacle_existence = Array2::from_elem(shape, false);
        let obstacle_map = Array2::from_elem(shape, ThinVec::new());

        let mut env = Environment {
            unit,
            shape,
            obstacle_existence,
            obstacle_map,
            potentials: Vec::new(),
        };

        for (i, obstacle) in scenario.obstacles.iter().enumerate() {
            env.add_obstacle(i, obstacle);
        }

        for (i, waypoint) in scenario.waypoints.iter().enumerate() {
            env.add_waypoint(i, waypoint);
        }

        env
    }

    fn add_obstacle(&mut self, index: usize, obstacle: &ObstacleConfig) {
        let line = obstacle.line;
        let shape = Line::from(line.map(|v| {
            let v = v / self.unit;
            (v.x, v.y)
        }));

        let mut rasterizer = BinaryBuilder::new()
            .width(self.shape.1)
            .height(self.shape.0)
            .build()
            .unwrap();
        rasterizer.rasterize(&shape).unwrap();
        let grid = rasterizer.finish();

        self.obstacle_existence.zip_mut_with(&grid, |a, b| *a |= b);

        self.obstacle_map.zip_mut_with(&grid, |map, exist| {
            if *exist {
                map.push(index as u32)
            }
        });
    }

    fn add_waypoint(&mut self, _index: usize, waypoint: &WaypointConfig) {
        let line = waypoint.line;
        let shape = Line::from(line.map(|v| {
            let v = v / self.unit;
            (v.x, v.y)
        }));

        let mut rasterizer = BinaryBuilder::new()
            .width(self.shape.1)
            .height(self.shape.0)
            .build()
            .unwrap();
        rasterizer.rasterize(&shape).unwrap();
        let grid = rasterizer.finish();

        let potential = self.calc_potential(grid);
        self.potentials.push(potential);
    }

    /// Calculate potential against a waypoint using Sethian's [fast marching method](https://en.wikipedia.org/wiki/Fast_marching_method).    
    fn calc_potential(&self, target_map: Array2<bool>) -> Array2<f32> {
        use Status::*;
        type Float = Reverse<NotNan<f32>>;

        #[derive(Debug, Clone, PartialEq, PartialOrd)]
        enum Status {
            Far,
            Considered,
            Accepted,
        }

        const F_DEF: f32 = 1.0;
        const F_OBS: f32 = 1e4;

        let mut potentials = Array2::from_elem(self.shape, f32::MAX);
        let mut states = Array2::from_elem(self.shape, Status::Far);
        let mut queue = BinaryHeap::<(Float, Index)>::new();
        let float = |x: f32| Reverse(NotNan::new(x).unwrap());

        for y in 0..self.shape.1 {
            for x in 0..self.shape.0 {
                let ix = Index::new(x, y);
                if target_map[ix] {
                    potentials[ix] = 0.0;
                    states[ix] = Accepted;

                    for (j, i) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                        let ix = ix.add(i, j);
                        if ix.is_inside(self.shape) && states[ix] != Accepted {
                            let u = if self.obstacle_existence[ix] {
                                F_OBS
                            } else {
                                F_DEF
                            };

                            potentials[ix] = u;
                            queue.push((float(u), ix));
                            states[ix] = Considered;
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

                if !ix.is_inside(self.shape) || states[ix] == Accepted {
                    continue;
                }
                states[ix] = Considered;

                let f = if self.obstacle_existence[ix] {
                    F_OBS
                } else {
                    F_DEF
                };

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
                    let sq = 2.0 * u1 * u2 - u1 * u1 - u2 * u2 + 2.0 * f * f;
                    if sq >= 0.0 {
                        (u1 + u2 + sq.sqrt()) / 2.0
                    } else {
                        f32::MAX
                    }
                };

                if u < potentials[ix] {
                    potentials[ix] = u;
                    queue.push((float(u), ix));
                }
            }
        }

        potentials
    }

    pub fn get_potential(&self, waypoint_id: usize, position: Vec2) -> f32 {
        let position = position / self.unit - Vec2::splat(0.5);
        let base = position.floor();
        let t = position - base;
        let b = base.as_ivec2();

        let potential = &self.potentials[waypoint_id];
        let shape = potential.shape();

        [(0, t.y), (1, 1.0 - t.y)]
            .iter()
            .map(|(dy, ty)| {
                [(0, t.x), (1, 1.0 - t.x)]
                    .iter()
                    .map(|(dx, tx)| {
                        let (x, y) = (b.x + dx, b.y + dy);
                        if y < 0 || y >= shape[0] as i32 || x < 0 || x >= shape[1] as i32 {
                            return 1e24;
                        }
                        potential[(y as usize, x as usize)] * tx
                    })
                    .sum::<f32>()
                    * ty
            })
            .sum()
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Index {
    pub y: i32,
    pub x: i32,
}

impl Index {
    pub fn new<T: PrimInt>(x: T, y: T) -> Self {
        Index {
            x: x.to_i32().unwrap(),
            y: y.to_i32().unwrap(),
        }
    }

    pub fn add<T: PrimInt>(self, x: T, y: T) -> Self {
        Index {
            x: self.x + x.to_i32().unwrap(),
            y: self.y + y.to_i32().unwrap(),
        }
    }

    pub fn is_inside(self, shape: (usize, usize)) -> bool {
        self.x >= 0 && self.x < shape.1 as i32 && self.y >= 0 && self.y < shape.0 as i32
    }
}

unsafe impl ndarray::NdIndex<ndarray::Ix2> for Index {
    fn index_checked(&self, dim: &ndarray::Ix2, strides: &ndarray::Ix2) -> Option<isize> {
        (self.y as usize, self.x as usize).index_checked(dim, strides)
    }

    fn index_unchecked(&self, strides: &ndarray::Ix2) -> isize {
        (self.y as usize, self.x as usize).index_unchecked(strides)
    }
}

#[cfg(test)]
mod tests {
    use geo::{LineString, Polygon};
    use geo_rasterize::BinaryBuilder;
    use glam::vec2;
    use ndarray::Array2;

    use crate::simulator::scenario::{FieldConfig, ObstacleConfig, Scenario, WaypointConfig};

    use super::Environment;

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
                },
                ObstacleConfig {
                    line: [vec2(1.0, 3.5), vec2(5.0, 3.5)],
                },
            ],
            waypoints: vec![WaypointConfig {
                line: [vec2(0.0, 0.0), vec2(0.0, 1.0)],
            }],
            ..Default::default()
        };

        let environment = Environment::from_scenario(&scenario);

        println!(
            "{:?}",
            environment
                .obstacle_existence
                .map(|v| if *v { 1 } else { 0 })
        );

        println!("{:?}", environment.potentials[0].map(|v| *v as i32));

        println!("{:?}", environment.get_potential(0, vec2(-1.5, 2.0)));

        // let mut target_map = Array2::from_elem(environment.shape, false);
        // target_map[(10, 5)] = true;
        // // target_map[(10, 6)] = true;
        // let potential = environment.calc_potential(target_map);

        // println!("{:#?}", potential.map(|v| *v as i32));
    }
}
