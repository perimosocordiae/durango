mod common;
mod greedy;
mod random;

pub use crate::agent::common::Agent;

pub fn create_agent(difficulty: usize) -> Box<dyn Agent + Send> {
    match difficulty {
        // Random (valid) actions.
        0 => Box::<random::RandomAgent>::default(),
        // Very simple heuristics.
        _ => Box::<greedy::GreedyAgent>::default(),
    }
}
