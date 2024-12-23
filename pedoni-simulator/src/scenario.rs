use glam::Vec2;
use serde::Deserialize;

const fn f_one() -> f32 {
    1.0
}

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

#[derive(Debug, Clone, Deserialize)]
pub struct ObstacleConfig {
    pub line: [Vec2; 2],
    #[serde(default = "f_one")]
    pub width: f32,
}

impl Default for ObstacleConfig {
    fn default() -> Self {
        ObstacleConfig {
            line: Default::default(),
            width: 1.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct WaypointConfig {
    pub line: [Vec2; 2],
    #[serde(default = "f_one")]
    pub width: f32,
}

impl Default for WaypointConfig {
    fn default() -> Self {
        WaypointConfig {
            line: Default::default(),
            width: 1.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PedestrianConfig {
    pub origin: usize,
    pub destination: usize,
    pub spawn: PedestrianSpawnConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PedestrianSpawnConfig {
    Periodic { frequency: f64 },
    Once { count: i32 },
}

#[derive(Debug, Default, Clone, Deserialize)]

pub enum PedestrianSpawnKind {
    #[default]
    Periodic,
    Once,
}
