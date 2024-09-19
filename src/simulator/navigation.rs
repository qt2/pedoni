use std::{cmp::Reverse, collections::BinaryHeap};

use geo::Line;
use geo_rasterize::BinaryBuilder;
use ndarray::{s, Array2};
use ordered_float::NotNan;
use thin_vec::{thin_vec, ThinVec};

use super::scenario::Scenario;

pub struct Environment {
    unit: f32,
    shape: (usize, usize),
    obstacles: Array2<bool>,
    obstacle_map: Array2<ThinVec<u32>>,
}

impl Environment {
    pub fn from_scenario(scenario: &Scenario) -> Self {
        let unit = 0.5;
        let size = (scenario.field.size / unit).ceil().as_ivec2();
        let shape = (size.y as usize, size.x as usize);
        let mut obstacles = Array2::from_elem(shape, false);
        let mut obstacle_map = Array2::from_elem(shape, thin_vec![]);

        for (i, obstacle) in scenario.obstacles.iter().enumerate() {
            let line = obstacle.line;
            let shape = Line::from(line.map(|v| {
                let v = v / unit;
                (v.x, v.y)
            }));

            let mut rasterizer = BinaryBuilder::new()
                .width(size.x as usize)
                .height(size.y as usize)
                .build()
                .unwrap();
            rasterizer.rasterize(&shape).unwrap();
            let grid = rasterizer.finish();

            obstacles
                .slice_mut(s![.., ..])
                .zip_mut_with(&grid, |a, b| *a |= b);

            obstacle_map.zip_mut_with(&grid, |map, exist| {
                if *exist {
                    map.push(i as u32)
                }
            });
        }

        Environment {
            unit,
            shape,
            obstacles,
            obstacle_map,
        }
    }

    fn calc_potential(&mut self, target_map: Array2<bool>) -> Array2<f32> {
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
            for (dj, di) in [(-1, -1), (-1, 1), (1, -1), (1, 1)] {
                match (j.checked_add_signed(dj), i.checked_add_signed(di)) {
                    (Some(y), Some(x)) => {
                        if status[(y, x)] != Accepted {
                            status[(y, x)] = Considered;
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
            environment.obstacles.map(|v| if *v { 1 } else { 0 })
        );
    }
}
