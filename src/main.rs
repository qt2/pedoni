pub mod renderer;
pub mod simulator;

use std::{
    fs,
    sync::{Mutex, RwLock},
    thread,
    time::{Duration, Instant},
};

use log::info;
use once_cell::sync::Lazy;
use simulator::diagnostic::{Diagnostic, DiagnosticMetrics};

use crate::{
    renderer::Renderer,
    simulator::{scenario::Scenario, Simulator},
};

static SIMULATOR: Lazy<RwLock<Simulator>> =
    Lazy::new(|| RwLock::new(Simulator::with_scenario(Scenario::default())));
static STATE: Mutex<State> = Mutex::new(State {
    paused: true,
    replay_requested: false,
    delta_time: 0.1,
    playback_speed: 4.0,
    use_neighbor_grid: false,
    neighbor_grid_unit: 2.0,
});
static DIAGNOSTIC: Lazy<Mutex<Diagnostic>> = Lazy::new(|| Mutex::new(Diagnostic::default()));

#[derive(Debug, Clone)]
pub struct State {
    pub paused: bool,
    pub replay_requested: bool,
    pub delta_time: f64,
    pub playback_speed: f64,
    pub use_neighbor_grid: bool,
    pub neighbor_grid_unit: f32,
}

fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_module("pedoni", log::LevelFilter::Info)
        .init();

    let scenario = fs::read_to_string("scenarios/default.toml")?;
    let scenario: Scenario = toml::from_str(&scenario)?;

    {
        let mut simulator = SIMULATOR.write().unwrap();
        *simulator = Simulator::with_scenario(scenario);
    }

    info!("successfully loaded a scenario");

    thread::spawn(move || loop {
        let start = Instant::now();
        let state = STATE.lock().unwrap().clone();

        if !state.paused {
            {
                let mut simulator = SIMULATOR.write().unwrap();
                simulator.spawn_pedestrians();
            }

            let (time_calc_state, next_state) = {
                let simulator = SIMULATOR.read().unwrap();
                (Instant::now(), simulator.calc_next_state())
            };
            let time_calc_state = time_calc_state.elapsed().as_secs_f64();
            let active_ped_count = next_state.len() as i32;

            let (time_apply_state, _) = {
                let mut simulator = SIMULATOR.write().unwrap();
                (Instant::now(), simulator.apply_next_state(next_state))
            };
            let time_apply_state = time_apply_state.elapsed().as_secs_f64();

            let mut diagnostic = DIAGNOSTIC.lock().unwrap();
            diagnostic.push(DiagnosticMetrics {
                active_ped_count,
                time_calc_state,
                time_apply_state,
            });
        }

        let calc_time = Instant::now() - start;
        let min_interval = Duration::from_secs_f64(state.delta_time / state.playback_speed);
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
