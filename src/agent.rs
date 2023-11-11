use crate::game::{GameState, PlayerAction};

pub trait Agent {
    fn choose_action(&self, game: &GameState) -> PlayerAction;
}

pub fn create_agent(difficulty: usize) -> Box<dyn Agent + Send> {
    match difficulty {
        // Random (valid) actions.
        0 => Box::<RandomAgent>::default(),
        _ => todo!("nontrivial agents"),
    }
}

#[derive(Default)]
struct RandomAgent {}
impl Agent for RandomAgent {
    fn choose_action(&self, game: &GameState) -> PlayerAction {
        let _me = game.curr_player();
        todo!("RandomAgent::choose_action")
    }
}
