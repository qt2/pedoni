use rustc_hash::FxHashMap;
use serde::Deserialize;

use glam::Vec2;

/// Scenario data
#[derive(Debug, Default, Deserialize)]
pub struct Scenario {
    pub walls: Vec<WallConfig>,
    pub waypoints: FxHashMap<String, WallConfig>,
    pub pedestrians: Vec<PedestrianConfig>,
}

#[derive(Debug, Default, Deserialize)]
pub struct WallConfig {
    pub polygon: Vec<Vec2>,
}

#[derive(Debug, Default, Deserialize)]
pub struct WaypointConfig {
    pub label: Option<String>,
    pub polygon: Vec<Vec2>,
}

#[derive(Debug, Default, Deserialize)]
pub struct PedestrianConfig {
    pub origin: usize,
    pub destination: usize,
    pub waypoints: Option<Vec<usize>>,
}
