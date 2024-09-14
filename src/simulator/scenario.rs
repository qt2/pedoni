use glam::Vec2;
use serde::Deserialize;

/// Scenario data
#[derive(Debug, Default, Clone, Deserialize)]
pub struct Scenario {
    pub field: FieldConfig,
    pub waypoints: Vec<WaypointConfig>,
    pub obstacles: Vec<ObstacleConfig>,
    pub pedestrians: Vec<PedestrianConfig>,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct FieldConfig {
    pub size: Vec2,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct ObstacleConfig {
    pub line: [Vec2; 2],
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct WaypointConfig {
    pub line: [Vec2; 2],
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct PedestrianConfig {
    pub origin: usize,
    pub destination: usize,
    pub spawn: PedestrianSpawnConfig,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct PedestrianSpawnConfig {
    pub kind: PedestrianSpawnKind,
    pub frequency: f64,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PedestrianSpawnKind {
    #[default]
    Periodic,
}
