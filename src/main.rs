pub mod renderer;
pub mod simulator;

use std::{
    fs,
    sync::RwLock,
    thread,
    time::{Duration, Instant},
};

use once_cell::sync::Lazy;

use crate::{
    renderer::Renderer,
    simulator::{scenario::Scenario, Simulator},
};

static SIMULATOR: Lazy<RwLock<Simulator>> =
    Lazy::new(|| RwLock::new(Simulator::with_scenario(Scenario::default())));

fn main() -> anyhow::Result<()> {
    let config = Config::default();
    let min_interval = Duration::from_secs_f32(config.delta_time / config.playback_speed);

    let scenario = fs::read_to_string("scenarios/default.toml")?;
    let scenario: Scenario = toml::from_str(&scenario)?;

    {
        let mut simulator = SIMULATOR.write().unwrap();
        *simulator = Simulator::with_scenario(scenario);
    }

    thread::spawn(move || loop {
        let start = Instant::now();

        let accels = {
            let simulator = SIMULATOR.read().unwrap();
            simulator.calc_acceleration()
        };

        {
            let mut simulator = SIMULATOR.write().unwrap();
            simulator.tick(accels);
        }

        let calc_time = Instant::now() - start;

        if calc_time < min_interval {
            thread::sleep(min_interval - calc_time);
        }
    });

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([960.0, 720.0]),
        renderer: eframe::Renderer::Wgpu,
        multisampling: 4,
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

pub struct Config {
    /// Delta time (in seconds)
    delta_time: f32,
    /// Max playback speed (1x)
    playback_speed: f32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            delta_time: 0.1,
            playback_speed: 1.0,
        }
    }
}
