use serde::Deserialize;

use crate::Vec2;

/// Scenario data
#[derive(Debug, Default, Deserialize)]
pub struct Scenario {
    pub walls: Vec<WallConfig>,
}

#[derive(Debug, Default, Deserialize)]
pub struct WallConfig {
    pub vertice: Vec<Vec2>,
}
