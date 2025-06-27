use glam::Vec2;

use crate::scenario::{ObstacleConfig, WaypointConfig};

#[derive(Debug, Default, Clone)]
pub struct MapData {
    pub size: Vec2,
    pub obstacles: Vec<ObstacleConfig>,
    pub waypoints: Vec<WaypointConfig>,
}
