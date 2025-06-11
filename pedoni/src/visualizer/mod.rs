use glam::Vec2;
use pedoni_simulator::{scenario::Scenario, Simulator};
use plotters::{
    coord::Shift,
    prelude::*,
    style::full_palette::{GREY_500, ORANGE},
};

const PALETTE: &[RGBColor] = &[BLUE, RED, GREEN, YELLOW, CYAN, MAGENTA];

pub struct VisualizerOptions {
    pub pixels_per_meter: f32,
    pub frame_interval: u32,
}

impl Default for VisualizerOptions {
    fn default() -> Self {
        VisualizerOptions {
            pixels_per_meter: 10.0,
            frame_interval: 100,
        }
    }
}

pub struct Visualizer<'a> {
    pub root: DrawingArea<BitMapBackend<'a>, Shift>,
    pub options: VisualizerOptions,
}

impl<'a> Visualizer<'a> {
    pub fn new(output_path: &str, scenario: &Scenario, options: VisualizerOptions) -> Self {
        let size = scenario.field.size;
        let resolution = (size * options.pixels_per_meter).ceil().as_uvec2();
        let root = BitMapBackend::gif(output_path, resolution.into(), options.frame_interval)
            .unwrap()
            .into_drawing_area();

        Visualizer { root, options }
    }

    pub fn render(&self, simulator: &Simulator) {
        let scenario = &simulator.scenario;

        self.root.fill(&WHITE).unwrap();

        for obstacle in &scenario.obstacles {
            let width = obstacle.width * self.options.pixels_per_meter;
            let style = GREY_500.stroke_width(width as u32);
            self.draw_path(&obstacle.line, style);
        }

        for destination in &scenario.waypoints {
            let width = 1.0 * self.options.pixels_per_meter;
            let style = ORANGE.stroke_width(width as u32);
            self.draw_path(&destination.line, style);
        }

        for pedestrian in &simulator.list_pedestrians() {
            let style = PALETTE[pedestrian.destination % PALETTE.len()].filled();
            self.draw_circle(pedestrian.pos, 0.2, style);
        }

        self.root.present().unwrap();
    }

    /// Draws a circle at the given position with the specified radius and style.
    /// The position is in world coordinates, and the radius is in world units.
    fn draw_circle(&self, position: Vec2, radius: f32, style: impl Into<ShapeStyle>) {
        let coord = (position * self.options.pixels_per_meter)
            .round()
            .as_ivec2();
        self.root
            .draw(&Circle::new(
                coord.into(),
                (radius * self.options.pixels_per_meter) as i32,
                style,
            ))
            .unwrap();
    }

    /// Draws a path defined by a series of points.
    /// The points are in world coordinates.
    fn draw_path(&self, points: &[Vec2], style: impl Into<ShapeStyle>) {
        let points: Vec<(i32, i32)> = points
            .iter()
            .map(|p| {
                (p * self.options.pixels_per_meter)
                    .round()
                    .as_ivec2()
                    .into()
            })
            .collect();
        self.root.draw(&PathElement::new(points, style)).unwrap();
    }
}
