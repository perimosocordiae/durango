use std::cell::OnceCell;

use crate::agent::common::*;
use crate::cards::Card;
use crate::data::Node;
use crate::game::{ActionOutcome, GameState, PlayerAction};
use crate::player::Player;

pub(super) struct TurnPlannerAgent {
    // Score single-hex distances as node.cost^cost_exponent.
    pub cost_exponent: i32,
    dists: OnceCell<Vec<f64>>,
}
impl Agent for TurnPlannerAgent {
    fn choose_action(&self, game: &GameState) -> PlayerAction {
        let (best, num_sims) = self.find_best_action(game);
        if num_sims >= 10000 {
            println!("Sims = {num_sims}\t Score = {}", best.score);
        }
        best.action
    }
}
impl TurnPlannerAgent {
    pub(super) fn new(cost_exponent: i32) -> Self {
        Self {
            cost_exponent,
            dists: OnceCell::new(),
        }
    }
    fn get_dists(&self, game: &GameState) -> &[f64] {
        self.dists.get_or_init(|| {
            if self.cost_exponent == 0 {
                return game.graph.dists.iter().map(|&d| d as f64).collect();
            }
            game.graph.distances_to_finish(&game.map, |node: &Node| {
                (node.cost as f64).powi(self.cost_exponent)
            })
        })
    }
    fn score_game_state(&self, game: &GameState) -> f64 {
        let me = game.curr_player();
        let my_idx = game.map.node_idx(me.position).unwrap();
        let dist_to_finish = self.get_dists(game)[my_idx];
        let num_tokens = me.tokens.len();
        let num_barriers = me.broken_barriers.len();
        let card_value = score_player_cards(me);
        // TODO: better scoring function
        card_value
            + (num_tokens as f64) * 10.0
            + (num_barriers as f64) * 100.0
            + dist_to_finish * -1000.0
    }

    fn find_best_action(&self, game: &GameState) -> (ActionScore, usize) {
        let num_cards = game.curr_player().hand.len();
        let mut best = ActionScore {
            action: if num_cards == 0 {
                PlayerAction::FinishTurn
            } else {
                PlayerAction::Discard((0..num_cards).collect())
            },
            score: self.score_game_state(game),
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
            let (res, ct) = self.find_best_action(&simulated_game);
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
}

struct ActionScore {
    action: PlayerAction,
    score: f64,
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

fn score_player_cards(player: &Player) -> f64 {
    let mut score = 0.0;
    for (card, count) in player.all_cards() {
        score += score_card(card) * (count as f64);
    }
    score
}

fn score_card(card: &Card) -> f64 {
    let movement_sum: u8 = card.movement.iter().sum();
    // TODO: factor in actions, etc.
    // TODO: weight move types differently based on upcoming terrain
    movement_sum as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_choose_action() {
        let game = GameState::new(2, "first", &mut rand::rng()).unwrap();
        let agent = TurnPlannerAgent::new(0);
        let action = agent.choose_action(&game);
        println!("Chosen action: {:?}", action);
    }
}
