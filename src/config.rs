use serde::{Deserialize, Serialize};

/// Configuration parameters for the simulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub grid_width: usize,
    pub grid_height: usize,
    pub initial_population: usize,
    pub payoff_t: f32, // Temptation
    pub payoff_r: f32, // Reward
    pub payoff_p: f32, // Punishment
    pub payoff_s: f32, // Sucker
    pub mutation_rate: f32,
    pub reproduction_threshold: f32,
    pub neighborhood_n: f32, // 0.0 to 1.0
    pub max_generations: usize,
    pub seed: u64,
    pub continuous_mode: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            grid_width: 100,
            grid_height: 100,
            initial_population: 10000,
            payoff_t: 5.0,
            payoff_r: 3.0,
            payoff_p: 1.0,
            payoff_s: 0.0,
            mutation_rate: 0.01,
            reproduction_threshold: 10.0,
            neighborhood_n: 1.0,
            max_generations: 1000,
            seed: 42,
            continuous_mode: false,
        }
    }
}
