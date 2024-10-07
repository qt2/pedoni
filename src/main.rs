pub mod renderer;
pub mod simulator;

use std::{
    fs::{self},
    sync::{Mutex, RwLock},
    thread,
    time::{Duration, Instant},
};

use log::{info, warn};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct State {
    pub paused: bool,
    pub replay_requested: bool,
    pub delta_time: f64,
    pub playback_speed: f64,
    pub use_neighbor_grid: bool,
    pub neighbor_grid_unit: f32,
}

const CONFIG_DIR: &str = ".pedoni";
const STATE_FILE: &str = "state.json";

fn load_state() {
    let config_dir = dirs::home_dir().unwrap().join(CONFIG_DIR);
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).unwrap();
    } else if !config_dir.is_dir() {
        panic!("{} is not a directory", config_dir.to_string_lossy());
    }

    let state_file = config_dir.join(STATE_FILE);
    if let Ok(text) = fs::read_to_string(&state_file) {
        if let Ok(state) = serde_json::from_str::<State>(&text) {
            *STATE.lock().unwrap() = state;
            info!("successfully loaded saved state");
        }
    }
}

fn save_state() {
    let state_file = dirs::home_dir().unwrap().join(CONFIG_DIR).join(STATE_FILE);
    let state = serde_json::to_string_pretty(&*STATE.lock().unwrap()).unwrap();
    if fs::write(&state_file, state).is_ok() {
        info!("successfully saved state");
    } else {
        warn!("failed to save state");
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_module("pedoni", log::LevelFilter::Info)
        .init();

    load_state();

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
        Box::new(|cc| Ok(Box::new(Renderer::new(cc)))),
    )
    .unwrap();

    save_state();

    Ok(())
}
