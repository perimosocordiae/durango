mod common;
mod greedy;
mod random;
mod turn_planner;

pub use crate::agent::common::Agent;

pub fn create_agent(difficulty: usize) -> Box<dyn Agent + Send> {
    match difficulty {
        // Random (valid) actions.
        0 => Box::<random::RandomAgent>::default(),
        // Very simple heuristics.
        1 => Box::<greedy::GreedyAgent>::default(),
        // Plans out all moves in a single turn.
        2 => Box::new(turn_planner::TurnPlannerAgent::new(0)),
        3 => Box::new(turn_planner::TurnPlannerAgent::new(1)),
        _ => Box::new(turn_planner::TurnPlannerAgent::new(2)),
    }
}
