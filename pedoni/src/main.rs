mod args;
pub mod renderer;

use std::{
    fs::{self, File},
    path::PathBuf,
    sync::{atomic::AtomicBool, Mutex, RwLock},
    thread,
    time::{Duration, Instant},
};

use args::Args;
use clap::Parser;
use log::{info, warn};
use once_cell::sync::Lazy;
use pedoni_simulator::{scenario::Scenario, Simulator, SimulatorOptions};

use crate::renderer::Renderer;

static SIMULATOR: Lazy<RwLock<Simulator>> = Lazy::new(|| RwLock::new(Simulator::new()));
static STATE: Mutex<State> = Mutex::new(State {
    paused: true,
    playback_speed: 4.0,
});
static SIG_INT: AtomicBool = AtomicBool::new(false);

pub const DELTA_TIME: f32 = 0.1;

#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub paused: bool,
    pub playback_speed: f32,
}

fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_module("pedoni", log::LevelFilter::Info)
        .init();

    if cfg!(debug_assertions) {
        warn!("Debug build");
    }

    let args = Args::parse();

    info!(
        "Model type: {:?} ({}), Backend: {:?}",
        args.model,
        if args.no_grid { "no grid" } else { "with grid" },
        args.backend,
    );

    STATE.lock().unwrap().playback_speed = args.speed;

    let scenario: Scenario = toml::from_str(&fs::read_to_string(&args.scenario)?)?;
    let field_size = scenario.field.size;
    info!("Loaded scenario file: {:?}", args.scenario.display());

    {
        let mut simulator = SIMULATOR.write().unwrap();
        let options = SimulatorOptions::default();
        simulator.initialize(scenario, &options);

        info!("Model initialization finished");
    }

    thread::spawn(move || loop {
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
                    if let Some(count) = diangostic_log.step_metrics.active_ped_count.last() {
                        info!(
                            "Step: {:6}, Active pedestrians: {:6}",
                            diangostic_log.total_steps, count
                        );
                    }
                }
            }
        }

        let step_time = Instant::now() - start;
        let min_interval = Duration::from_secs_f32(DELTA_TIME / state.playback_speed);
        if step_time < min_interval {
            thread::sleep(min_interval - step_time);
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

                serde_json::to_writer(&mut log_file, &simulator.diagnostic_log)?;
                info!("Exported log file: {}", log_path.display());

                break;
            }

            thread::sleep(Duration::from_millis(100));
        }
    } else {
        pittore::run("Pedoni", Renderer::new(field_size));
    }

    Ok(())
}
