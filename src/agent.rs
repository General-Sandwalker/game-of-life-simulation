use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Move {
    Cooperate,
    Defect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Strategy {
    AlwaysCooperate,
    AlwaysDefect,
    TitForTat,
    GrimTrigger,
    Pavlov,
    Random,
}

impl Strategy {
    pub fn all() -> Vec<Strategy> {
        vec![
            Strategy::AlwaysCooperate,
            Strategy::AlwaysDefect,
            Strategy::TitForTat,
            Strategy::GrimTrigger,
            Strategy::Pavlov,
            Strategy::Random,
        ]
    }

    pub fn color(&self) -> [u8; 3] {
        match self {
            Strategy::AlwaysCooperate => [0, 255, 0], // Green
            Strategy::AlwaysDefect => [255, 0, 0],    // Red
            Strategy::TitForTat => [0, 0, 255],       // Blue
            Strategy::GrimTrigger => [255, 165, 0],   // Orange
            Strategy::Pavlov => [128, 0, 128],        // Purple
            Strategy::Random => [128, 128, 128],      // Gray
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InteractionHistory {
    pub opponent_last_move: Option<Move>,
    pub my_last_move: Option<Move>,
    pub last_outcome: f32,
    pub round_number: usize,
    pub opponent_defected_ever: bool, // Useful for Grim Trigger
}

impl Default for InteractionHistory {
    fn default() -> Self {
        Self {
            opponent_last_move: None,
            my_last_move: None,
            last_outcome: 0.0,
            round_number: 0,
            opponent_defected_ever: false,
        }
    }
}

/// Represents a single agent in the simulation.
#[derive(Clone, Debug)]
pub struct Agent {
    pub strategy: Strategy,
    pub payoff: f32,
    pub history: HashMap<usize, InteractionHistory>,
    pub age: usize,
}

impl Agent {
    /// Creates a new agent with the given strategy.
    pub fn new(_id: usize, strategy: Strategy) -> Self {
        Self {
            strategy,
            payoff: 0.0,
            history: HashMap::new(),
            age: 0,
        }
    }

    pub fn decide_move(&self, opponent_id: usize, _round: usize) -> Move {
        let history = self.history.get(&opponent_id);

        match self.strategy {
            Strategy::AlwaysCooperate => Move::Cooperate,
            Strategy::AlwaysDefect => Move::Defect,
            Strategy::TitForTat => {
                if let Some(h) = history {
                    h.opponent_last_move.unwrap_or(Move::Cooperate)
                } else {
                    Move::Cooperate
                }
            }
            Strategy::GrimTrigger => {
                if let Some(h) = history {
                    if h.opponent_defected_ever {
                        Move::Defect
                    } else {
                        Move::Cooperate
                    }
                } else {
                    Move::Cooperate
                }
            }
            Strategy::Pavlov => {
                // Win-Stay, Lose-Shift (true Pavlov):
                // If opponent cooperated last round → keep my last move (I "won" or at least did OK)
                // If opponent defected last round → switch my move (I was exploited or stuck in DD)
                // Outcomes: (C,C)=R→stay=C; (D,C)=T→stay=D; (D,D)=P→shift=C; (C,D)=S→shift=D
                if let Some(h) = history {
                    if let (Some(my), Some(opp)) = (h.my_last_move, h.opponent_last_move) {
                        match opp {
                            Move::Cooperate => my,          // opponent cooperated → keep my move
                            Move::Defect => match my {      // opponent defected → switch
                                Move::Cooperate => Move::Defect,
                                Move::Defect => Move::Cooperate,
                            },
                        }
                    } else {
                        Move::Cooperate
                    }
                } else {
                    Move::Cooperate
                }
            }
            Strategy::Random => {
                let mut rng = rand::thread_rng();
                if rng.gen_bool(0.5) {
                    Move::Cooperate
                } else {
                    Move::Defect
                }
            }
        }
    }

    pub fn update_history(
        &mut self,
        opponent_id: usize,
        my_move: Move,
        opponent_move: Move,
        payoff: f32,
        round: usize,
    ) {
        self.payoff += payoff;
        let entry = self.history.entry(opponent_id).or_default();
        entry.my_last_move = Some(my_move);
        entry.opponent_last_move = Some(opponent_move);
        entry.last_outcome = payoff;
        entry.round_number = round;
        if opponent_move == Move::Defect {
            entry.opponent_defected_ever = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_always_cooperate() {
        let agent = Agent::new(0, Strategy::AlwaysCooperate);
        assert_eq!(agent.decide_move(1, 0), Move::Cooperate);
    }

    #[test]
    fn test_always_defect() {
        let agent = Agent::new(0, Strategy::AlwaysDefect);
        assert_eq!(agent.decide_move(1, 0), Move::Defect);
    }

    #[test]
    fn test_tit_for_tat() {
        let mut agent = Agent::new(0, Strategy::TitForTat);
        assert_eq!(agent.decide_move(1, 0), Move::Cooperate);
        
        agent.update_history(1, Move::Cooperate, Move::Defect, 0.0, 0);
        assert_eq!(agent.decide_move(1, 1), Move::Defect);
        
        agent.update_history(1, Move::Defect, Move::Cooperate, 5.0, 1);
        assert_eq!(agent.decide_move(1, 2), Move::Cooperate);
    }

    #[test]
    fn test_grim_trigger() {
        let mut agent = Agent::new(0, Strategy::GrimTrigger);
        assert_eq!(agent.decide_move(1, 0), Move::Cooperate);
        
        agent.update_history(1, Move::Cooperate, Move::Cooperate, 3.0, 0);
        assert_eq!(agent.decide_move(1, 1), Move::Cooperate);
        
        agent.update_history(1, Move::Cooperate, Move::Defect, 0.0, 1);
        assert_eq!(agent.decide_move(1, 2), Move::Defect);
        
        agent.update_history(1, Move::Defect, Move::Cooperate, 5.0, 2);
        assert_eq!(agent.decide_move(1, 3), Move::Defect);
    }

    #[test]
    fn test_pavlov() {
        let mut agent = Agent::new(0, Strategy::Pavlov);
        // First move: cooperate
        assert_eq!(agent.decide_move(1, 0), Move::Cooperate);

        // (C, C) → keep C
        agent.update_history(1, Move::Cooperate, Move::Cooperate, 3.0, 0);
        assert_eq!(agent.decide_move(1, 1), Move::Cooperate);

        // (C, D) → switch to D
        agent.update_history(1, Move::Cooperate, Move::Defect, 0.0, 1);
        assert_eq!(agent.decide_move(1, 2), Move::Defect);

        // (D, D) → switch to C
        agent.update_history(1, Move::Defect, Move::Defect, 1.0, 2);
        assert_eq!(agent.decide_move(1, 3), Move::Cooperate);

        // (D, C) → keep D (reset to defect state)
        agent.update_history(1, Move::Defect, Move::Cooperate, 5.0, 3);
        // Re-set last move state: we need an agent that last played D vs C opponent
        let mut agent2 = Agent::new(0, Strategy::Pavlov);
        agent2.update_history(1, Move::Defect, Move::Cooperate, 5.0, 0);
        assert_eq!(agent2.decide_move(1, 1), Move::Defect);
    }
}
