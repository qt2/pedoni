mod args;
pub mod renderer;
pub mod simulator;

use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Mutex, RwLock},
    thread,
    time::{Duration, Instant},
};

use args::Args;
use clap::Parser;
use log::{info, warn};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::{
    renderer::Renderer,
    simulator::{scenario::Scenario, Simulator},
};

static SIMULATOR: Lazy<RwLock<Simulator>> = Lazy::new(|| RwLock::new(Simulator::new()));
static STATE: Mutex<State> = Mutex::new(State {
    scenario_path: None,
    paused: true,
    replay_requested: false,
    delta_time: 0.1,
    playback_speed: 4.0,
    use_neighbor_grid: false,
    neighbor_grid_unit: 2.0,
});
// static DIAGNOSTIC: Lazy<Mutex<Diagnostic>> = Lazy::new(|| Mutex::new(Diagnostic::default()));
static SIG_INT: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct State {
    pub scenario_path: Option<PathBuf>,
    pub paused: bool,
    pub replay_requested: bool,
    pub delta_time: f64,
    pub playback_speed: f64,
    pub use_neighbor_grid: bool,
    pub neighbor_grid_unit: f32,
}

const CONFIG_DIR: &str = ".pedoni";
const STATE_FILE: &str = "state.json";

pub fn load_state() {
    let config_dir = dirs::home_dir().unwrap().join(CONFIG_DIR);
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).unwrap();
    } else if !config_dir.is_dir() {
        panic!("{} is not a directory", config_dir.to_string_lossy());
    }

    let state_file = config_dir.join(STATE_FILE);
    if let Ok(text) = fs::read_to_string(&state_file) {
        if let Ok(state) = serde_json::from_str::<State>(&text) {
            if let Some(ref path) = state.scenario_path {
                load_scenario(path).ok();
            }
            *STATE.lock().unwrap() = state;
            info!("successfully loaded saved state");
        }
    }
}

pub fn save_state() {
    let state_file = dirs::home_dir().unwrap().join(CONFIG_DIR).join(STATE_FILE);
    let state = serde_json::to_string_pretty(&*STATE.lock().unwrap()).unwrap();
    if fs::write(&state_file, state).is_ok() {
        info!("successfully saved state");
    } else {
        warn!("failed to save state");
    }
}

pub fn load_scenario(path: impl AsRef<Path>) -> anyhow::Result<()> {
    let scenario = fs::read_to_string(&path)?;
    let scenario: Scenario = toml::from_str(&scenario)?;
    {
        let mut simulator = SIMULATOR.write().unwrap();
        simulator.load_scenario(scenario);
    }
    STATE.lock().unwrap().scenario_path = path.as_ref().canonicalize().ok();
    info!("successfully loaded a scenario: {:?}", path.as_ref());

    Ok(())
}

fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_module("pedoni", log::LevelFilter::Info)
        .init();

    let args = Args::parse();

    load_state();

    thread::spawn(move || {
        {
            SIMULATOR.write().unwrap().initialize_model();
        }

        loop {
            let start = Instant::now();
            let state = STATE.lock().unwrap().clone();

            if !state.paused {
                {
                    let mut simulator = SIMULATOR.write().unwrap();
                    simulator.spawn_pedestrians();
                }
                {
                    let simulator = SIMULATOR.read().unwrap();
                    simulator.calc_next_state();
                }
                {
                    let mut simulator = SIMULATOR.write().unwrap();
                    simulator.apply_next_state();
                    simulator.collect_diagnostic_metrics();

                    let diangostic_log = &simulator.diagnostic_log;
                    if diangostic_log.total_steps % 100 == 0 {
                        if let Some(metrics) = diangostic_log.step_metrics.last() {
                            info!(
                                "Step: {:6}, Active pedestrians: {:6}",
                                diangostic_log.total_steps, metrics.active_ped_count
                            );
                        }
                    }
                }
            }

            let calc_time = Instant::now() - start;
            let min_interval = Duration::from_secs_f64(state.delta_time / state.playback_speed);
            if calc_time < min_interval {
                thread::sleep(min_interval - calc_time);
            }
        }
    });

    if args.headless {
        info!("Run as headless mode");
        ctrlc::set_handler(|| SIG_INT.store(true, std::sync::atomic::Ordering::SeqCst))?;

        STATE.lock().unwrap().paused = false;

        loop {
            if SIG_INT.load(std::sync::atomic::Ordering::SeqCst) {
                let current_time = chrono::Local::now();
                fs::create_dir("logs").ok();
                let log_path: PathBuf = [
                    "logs",
                    &current_time.format("%Y-%m-%d_%H%M%S_log.json").to_string(),
                ]
                .iter()
                .collect();
                let mut log_file = File::create(&log_path)?;
                let simulator = SIMULATOR.read().unwrap();

                serde_json::to_writer_pretty(&mut log_file, &simulator.diagnostic_log)?;
                info!("Exported log file: {}", log_path.display());

                break;
            }

            thread::sleep(Duration::from_millis(100));
        }
    } else {
        eframe::run_native(
            "Pedoni",
            pittore::pittore_eframe_options(),
            Box::new(|cc| Ok(Box::new(Renderer::new(cc)))),
        )
        .unwrap();

        save_state();
    }

    Ok(())
}
