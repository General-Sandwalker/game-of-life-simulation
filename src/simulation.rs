use crate::agent::{Agent, Move, Strategy};
use crate::config::Config;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use rayon::prelude::*;
use std::collections::HashMap;

/// The core simulation engine that manages the grid of agents and their interactions.
pub struct Simulation {
    pub config: Config,
    pub agents: Vec<Agent>,
    pub generation: usize,
}

impl Simulation {
    /// Creates a new simulation with the given configuration.
    /// Initializes the grid with strategies using a seeded RNG for reproducibility.
    pub fn new(config: Config) -> Self {
        let mut rng = StdRng::seed_from_u64(config.seed);
        let total_cells = config.grid_width * config.grid_height;
        let mut agents = Vec::with_capacity(total_cells);

        let strategies = Strategy::all();

        for i in 0..total_cells {
            let strategy = *strategies.choose(&mut rng).unwrap();
            agents.push(Agent::new(i, strategy));
        }

        Self {
            config,
            agents,
            generation: 0,
        }
    }

    pub fn step(&mut self) {
        let mut rng = rand::thread_rng();
        let n_agents = self.agents.len();
        let mut interactions = Vec::new();

        // Determine interactions
        for i in 0..n_agents {
            let x = i % self.config.grid_width;
            let y = i / self.config.grid_width;

            let neighbors = self.get_half_neighbors(x, y);

            for &neighbor_idx in &neighbors {
                let opponent_idx = if rng.gen::<f32>() < self.config.neighborhood_n {
                    neighbor_idx
                } else {
                    rng.gen_range(0..n_agents)
                };

                interactions.push((i, opponent_idx));
            }
        }

        // Play games
        let results: Vec<_> = interactions.par_iter().map(|&(i, j)| {
            if i == j {
                return None;
            }

            let move_i = self.agents[i].decide_move(j, self.generation);
            let move_j = self.agents[j].decide_move(i, self.generation);

            let (payoff_i, payoff_j) = self.calculate_payoff(move_i, move_j);

            Some((i, j, move_i, move_j, payoff_i, payoff_j))
        }).collect();

        let mut payoffs = vec![0.0; n_agents];
        let mut history_updates = vec![Vec::new(); n_agents];

        for res in results.into_iter().flatten() {
            let (i, j, move_i, move_j, payoff_i, payoff_j) = res;
            payoffs[i] += payoff_i;
            payoffs[j] += payoff_j;

            history_updates[i].push((j, move_i, move_j, payoff_i));
            history_updates[j].push((i, move_j, move_i, payoff_j));
        }

        // Apply updates
        for i in 0..n_agents {
            self.agents[i].age += 1;
            for (opp, my_move, opp_move, payoff) in history_updates[i].drain(..) {
                self.agents[i].update_history(opp, my_move, opp_move, payoff, self.generation);
            }
        }

        // Reproduction phase
        self.reproduce();

        self.generation += 1;
    }

    fn get_half_neighbors(&self, x: usize, y: usize) -> Vec<usize> {
        let mut neighbors = Vec::with_capacity(4);
        let w = self.config.grid_width as isize;
        let h = self.config.grid_height as isize;
        let x = x as isize;
        let y = y as isize;

        let offsets = [(1, 0), (1, 1), (0, 1), (-1, 1)];
        for (dx, dy) in offsets {
            let nx = (x + dx).rem_euclid(w) as usize;
            let ny = (y + dy).rem_euclid(h) as usize;
            neighbors.push(ny * self.config.grid_width + nx);
        }
        neighbors
    }

    fn get_neighbors(&self, x: usize, y: usize) -> Vec<usize> {
        let mut neighbors = Vec::with_capacity(8);
        let w = self.config.grid_width as isize;
        let h = self.config.grid_height as isize;
        let x = x as isize;
        let y = y as isize;

        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = (x + dx).rem_euclid(w) as usize;
                let ny = (y + dy).rem_euclid(h) as usize;
                neighbors.push(ny * self.config.grid_width + nx);
            }
        }
        neighbors
    }

    fn calculate_payoff(&self, move1: Move, move2: Move) -> (f32, f32) {
        match (move1, move2) {
            (Move::Cooperate, Move::Cooperate) => (self.config.payoff_r, self.config.payoff_r),
            (Move::Defect, Move::Defect) => (self.config.payoff_p, self.config.payoff_p),
            (Move::Cooperate, Move::Defect) => (self.config.payoff_s, self.config.payoff_t),
            (Move::Defect, Move::Cooperate) => (self.config.payoff_t, self.config.payoff_s),
        }
    }

    fn reproduce(&mut self) {
        // Simple evolutionary step:
        // For each cell, look at its neighbors (including itself).
        // The cell takes the strategy of the neighbor with the highest payoff.
        // Then apply mutation.
        
        let next_strategies: Vec<Strategy> = (0..self.agents.len()).into_par_iter().map(|i| {
            let mut rng = rand::thread_rng();
            let strategies = Strategy::all();
            let x = i % self.config.grid_width;
            let y = i / self.config.grid_width;
            let neighbors = self.get_neighbors(x, y);

            let mut best_idx = i;
            let mut best_payoff = self.agents[i].payoff;

            for &n in &neighbors {
                if self.agents[n].payoff > best_payoff {
                    best_payoff = self.agents[n].payoff;
                    best_idx = n;
                }
            }

            let mut new_strategy = self.agents[best_idx].strategy;

            if rng.gen::<f32>() < self.config.mutation_rate {
                new_strategy = *strategies.choose(&mut rng).unwrap();
            }

            new_strategy
        }).collect();

        // Apply new strategies and reset payoffs
        for i in 0..self.agents.len() {
            if self.agents[i].strategy != next_strategies[i] {
                self.agents[i].strategy = next_strategies[i];
                self.agents[i].history.clear(); // Reset memory on strategy change
                self.agents[i].age = 0;
            }
            self.agents[i].payoff = 0.0; // Reset payoff for next generation
        }
    }

    pub fn get_strategy_counts(&self) -> HashMap<Strategy, usize> {
        let mut counts = HashMap::new();
        for agent in &self.agents {
            *counts.entry(agent.strategy).or_insert(0) += 1;
        }
        counts
    }

    /// Returns (total_payoff, count) per strategy, for computing averages.
    pub fn get_avg_payoffs_by_strategy(&self) -> HashMap<Strategy, f32> {
        let mut totals: HashMap<Strategy, (f32, usize)> = HashMap::new();
        for agent in &self.agents {
            let entry = totals.entry(agent.strategy).or_insert((0.0, 0));
            entry.0 += agent.payoff;
            entry.1 += 1;
        }
        totals
            .into_iter()
            .map(|(s, (sum, n))| (s, if n > 0 { sum / n as f32 } else { 0.0 }))
            .collect()
    }

    /// Returns the global cooperation rate: fraction of cooperating moves in the last generation.
    /// Approximated by looking at agent histories — counts last recorded moves as C or D.
    pub fn get_cooperation_rate(&self) -> f32 {
        let (mut coops, mut total) = (0usize, 0usize);
        for agent in &self.agents {
            for h in agent.history.values() {
                if let Some(m) = h.my_last_move {
                    total += 1;
                    if m == Move::Cooperate {
                        coops += 1;
                    }
                }
            }
        }
        if total > 0 { coops as f32 / total as f32 } else { 0.0 }
    }

    /// Validates the payoff matrix satisfies Prisoner's Dilemma conditions:
    /// T > R > P > S  and  2R > T + S
    pub fn validate_payoffs(&self) -> Option<String> {
        let c = &self.config;
        if !(c.payoff_t > c.payoff_r) {
            return Some("Need T > R".to_string());
        }
        if !(c.payoff_r > c.payoff_p) {
            return Some("Need R > P".to_string());
        }
        if !(c.payoff_p > c.payoff_s) {
            return Some("Need P > S".to_string());
        }
        if !(2.0 * c.payoff_r > c.payoff_t + c.payoff_s) {
            return Some("Need 2R > T+S".to_string());
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_payoff() {
        let config = Config::default();
        let sim = Simulation::new(config);
        
        assert_eq!(sim.calculate_payoff(Move::Cooperate, Move::Cooperate), (3.0, 3.0));
        assert_eq!(sim.calculate_payoff(Move::Defect, Move::Defect), (1.0, 1.0));
        assert_eq!(sim.calculate_payoff(Move::Cooperate, Move::Defect), (0.0, 5.0));
        assert_eq!(sim.calculate_payoff(Move::Defect, Move::Cooperate), (5.0, 0.0));
    }

    #[test]
    fn test_get_neighbors() {
        let mut config = Config::default();
        config.grid_width = 3;
        config.grid_height = 3;
        let sim = Simulation::new(config);
        
        let neighbors = sim.get_neighbors(1, 1);
        assert_eq!(neighbors.len(), 8);
        
        let half_neighbors = sim.get_half_neighbors(1, 1);
        assert_eq!(half_neighbors.len(), 4);
    }
}
