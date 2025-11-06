use crate::agent::common::*;
use crate::game::{ActionOutcome, GameState, PlayerAction};
use crate::player::Player;

#[derive(Default)]
pub(super) struct TurnPlannerAgent {}
impl Agent for TurnPlannerAgent {
    fn choose_action(&self, game: &GameState) -> PlayerAction {
        let (best, num_sims) = find_best_action(game);
        if num_sims >= 10000 {
            println!("Sims = {num_sims}\t Score = {}", best.score);
        }
        best.action
    }
}

struct ActionScore {
    action: PlayerAction,
    score: f64,
}

fn find_best_action(game: &GameState) -> (ActionScore, usize) {
    let num_cards = game.curr_player().hand.len();
    let mut best = ActionScore {
        action: if num_cards == 0 {
            PlayerAction::FinishTurn
        } else {
            PlayerAction::Discard((0..num_cards).collect())
        },
        score: score_game_state(game),
    };

    let mut num_sims = 0;
    for action in all_actions(game) {
        let mut simulated_game = game.clone();
        let outcome = simulated_game.process_action(&action).unwrap();
        num_sims += 1;
        // If this ends the game, no need to keep going.
        if matches!(outcome, ActionOutcome::GameOver) {
            return (
                ActionScore {
                    action,
                    score: f64::MAX,
                },
                num_sims,
            );
        }
        // Otherwise, recurse.
        let (res, ct) = find_best_action(&simulated_game);
        num_sims += ct;
        if res.score > best.score {
            best = ActionScore {
                action,
                score: res.score,
            };
        }
    }
    (best, num_sims)
}

fn all_actions(game: &GameState) -> Vec<PlayerAction> {
    let mut actions = Vec::new();
    for draw in valid_draw_actions(game) {
        actions.push(PlayerAction::Draw(draw));
    }
    for buy in valid_buy_actions(game) {
        actions.push(PlayerAction::BuyCard(buy));
    }
    for mv in valid_move_actions(game) {
        actions.push(PlayerAction::Move(mv));
    }
    let me = game.curr_player();
    if can_safely_trash(me) {
        for i in 0..me.hand.len() {
            actions.push(PlayerAction::Trash(vec![i]));
        }
    }
    actions
}

fn score_game_state(game: &GameState) -> f64 {
    let me = game.curr_player();
    let my_idx = game.map.node_idx(me.position).unwrap();
    let dist_to_finish = game.graph.dists[my_idx];
    let num_tokens = me.tokens.len();
    let num_barriers = me.broken_barriers.len();
    let card_value = score_player_cards(me);
    // TODO: better scoring function
    card_value
        + (num_tokens as f64) * 10.0
        + (num_barriers as f64) * 100.0
        + (dist_to_finish as f64) * -1000.0
}

fn score_player_cards(player: &Player) -> f64 {
    // TODO: better scoring
    player.sum_movement().iter().sum::<u8>() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_choose_action() {
        let game = GameState::new(2, "first", &mut rand::rng()).unwrap();
        let agent = TurnPlannerAgent::default();
        let action = agent.choose_action(&game);
        println!("Chosen action: {:?}", action);
    }
}
