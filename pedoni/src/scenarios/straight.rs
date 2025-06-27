use pedoni_simulator::Simulator;

use super::Scenario;

pub struct StraitScenario;

impl Scenario for StraitScenario {
    fn initialize(&mut self, sim: &mut Simulator) {
        // Initialize the scenario with the simulator
        sim.model.spawn_pedestrians(&sim.field, vec![]);
    }

    fn update(&mut self, sim: &mut Simulator) {
        // Update the scenario state
        sim.step += 1;
        sim.model.update(&sim.field);
    }
}
