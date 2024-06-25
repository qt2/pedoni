pub mod scenario;

use self::scenario::Scenario;
use crate::Vec2;

/// Simulator instance
#[derive(Debug)]
pub struct Simulator {
    pub scenario: Scenario,
    pub walls: Vec<Wall>,
}

impl Simulator {
    /// Create new simulator instance with scenario
    pub fn new(scenario: Scenario) -> Self {
        let walls = scenario
            .walls
            .iter()
            .map(|c| Wall {
                pos: c.vertice.clone(),
            })
            .collect();

        Simulator { scenario, walls }
    }

    /// Tick and update environment
    pub fn tick(&mut self) {}
}

/// Wall instance
#[derive(Debug)]
pub struct Wall {
    pub pos: Vec<Vec2>,
}
