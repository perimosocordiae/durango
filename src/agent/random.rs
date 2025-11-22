use crate::agent::common::*;
use crate::game::{GameState, PlayerAction};
use rand::{Rng, RngCore};

#[derive(Default)]
pub(super) struct RandomAgent {}
impl Agent for RandomAgent {
    fn choose_action(
        &self,
        game: &GameState,
        rng: &mut dyn RngCore,
    ) -> PlayerAction {
        let me = game.curr_player();
        let mut valid_draws = valid_draw_actions(game);
        if !valid_draws.is_empty() {
            let idx = rng.random_range(0..valid_draws.len());
            return PlayerAction::Draw(valid_draws.swap_remove(idx));
        }
        if can_safely_trash(me) {
            let idx = rng.random_range(0..me.hand.len());
            return PlayerAction::Trash(vec![idx]);
        }
        let mut valid_moves = valid_move_actions(game);
        if !valid_moves.is_empty() {
            let idx = rng.random_range(0..valid_moves.len());
            return PlayerAction::Move(valid_moves.swap_remove(idx));
        }
        let mut valid_buys = valid_buy_actions(game);
        if !valid_buys.is_empty() {
            let idx = rng.random_range(0..valid_buys.len());
            return PlayerAction::BuyCard(valid_buys.swap_remove(idx));
        }
        if me.hand.is_empty() {
            return PlayerAction::FinishTurn;
        }
        PlayerAction::Discard((0..me.hand.len()).collect())
    }
}
