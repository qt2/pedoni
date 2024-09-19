use std::{cmp::Reverse, collections::BinaryHeap};

use geo::Line;
use geo_rasterize::BinaryBuilder;
use ndarray::{s, Array2};
use ordered_float::NotNan;
use thin_vec::ThinVec;

use super::scenario::{ObstacleConfig, Scenario, WaypointConfig};

pub struct Environment {
    /// Unit of length (in meters)
    unit: f32,
    /// Shape of 2D grid (y, x)
    shape: (usize, usize),
    /// Boolean grid which holds obstacle existence
    obstacle_existence: Array2<bool>,
    /// Vector grid indicating which obstacles each cell overlaps
    obstacle_map: Array2<ThinVec<u32>>,
    /// Potential against each waypoint
    potentials: Vec<Array2<f32>>,
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
        let unit = 0.5;
        let size = (scenario.field.size / unit).ceil().as_ivec2();
        let shape = (size.y as usize, size.x as usize);
        let mut env = Environment {
            unit,
            shape,
            ..Default::default()
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

        self.obstacle_existence
            .slice_mut(s![.., ..])
            .zip_mut_with(&grid, |a, b| *a |= b);

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

        let mut potential = Array2::from_elem(self.shape, f32::MAX);
        let mut status = Array2::from_elem(self.shape, Status::Far);
        let mut queue = BinaryHeap::<(Float, (usize, usize))>::new();
        let float = |x: f32| Reverse(NotNan::new(x).unwrap());

        for j in 0..self.shape.1 {
            for i in 0..self.shape.0 {
                if let Some(&v) = target_map.get((j, i)) {
                    if v {
                        potential[(j, i)] = 0.0;
                        status[(j, i)] = Accepted;
                        queue.push((float(0.0), (j, i)));
                    }
                }
            }
        }

        while let Some((u, (j, i))) = queue.pop() {
            let u = u.0.into_inner();
            status[(j, i)] = Accepted;

            for (dj, di) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                match (j.checked_add_signed(dj), i.checked_add_signed(di)) {
                    (Some(y), Some(x)) => {
                        if status.get((y, x)).is_none() {
                            continue;
                        }

                        if status[(y, x)] != Accepted {
                            status[(y, x)] = Considered;
                            let v = u + if self.obstacle_existence[(y, x)] {
                                1024.0
                            } else {
                                1.0
                            };

                            if potential[(y, x)] > v {
                                potential[(y, x)] = v;
                                queue.push((float(v), (y, x)));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        potential
    }
}

#[cfg(test)]
mod tests {
    use geo::{LineString, Polygon};
    use geo_rasterize::BinaryBuilder;
    use glam::vec2;
    use ndarray::Array2;

    use crate::simulator::scenario::{FieldConfig, ObstacleConfig, Scenario};

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
                size: vec2(20.0, 10.0),
            },
            obstacles: vec![
                ObstacleConfig {
                    line: [vec2(5.0, 4.0), vec2(15.0, 4.0)],
                },
                ObstacleConfig {
                    line: [vec2(5.0, 6.0), vec2(15.0, 6.0)],
                },
            ],
            ..Default::default()
        };

        let environment = Environment::from_scenario(&scenario);

        println!(
            "{:#?}",
            environment
                .obstacle_existence
                .map(|v| if *v { 1 } else { 0 })
        );

        let mut target_map = Array2::from_elem(environment.shape, false);
        target_map[(10, 5)] = true;
        // target_map[(10, 6)] = true;
        let potential = environment.calc_potential(target_map);

        println!("{:#?}", potential.map(|v| *v as i32));
    }
}
