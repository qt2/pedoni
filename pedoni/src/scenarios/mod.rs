use pedoni_simulator::Simulator;

// pub mod straight;

pub trait Scenario {
    /// Initializes the scenario.
    fn initialize(&mut self, sim: &mut Simulator);

    /// Updates the scenario state.
    fn update(&mut self, sim: &mut Simulator);
}
