pub mod renderer;
pub mod simulator;

use std::{
    fs,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use clap::Parser;
use eframe::wgpu::DeviceDescriptor;
use egui_wgpu::WgpuConfiguration;
use nalgebra::{Vector2, Vector3};

use crate::{
    renderer::Renderer,
    simulator::{scenario::Scenario, Simulator},
};

pub type Vec2 = Vector2<f32>;
pub type Vec3 = Vector3<f32>;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let scenario = fs::read_to_string(&args.scenario)?;
    let scenario: Scenario = toml::from_str(&scenario)?;

    let mut simulator = Simulator::new(scenario);

    let min_interval = Duration::from_secs_f32(100.0 * args.delta_time / args.playback_speed);

    thread::spawn(move || loop {
        let start = Instant::now();

        simulator.tick();

        let calc_time = Instant::now() - start;

        if calc_time < min_interval {
            thread::sleep(min_interval - calc_time);
        }
    });

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([960.0, 720.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "Pedoni",
        options,
        Box::new(|cc| Box::new(Renderer::new(cc))),
    )
    .unwrap();

    Ok(())
}

#[derive(Parser, Debug)]
struct Args {
    /// Scenario file
    #[arg(default_value = "scenarios/default.toml")]
    scenario: String,

    /// Delta time (in milliseconds)
    #[arg(short, long, default_value_t = 100.0)]
    delta_time: f32,

    /// Max playback speed (default: 10x)
    #[arg(short, long, default_value_t = 10.0)]
    playback_speed: f32,
}
