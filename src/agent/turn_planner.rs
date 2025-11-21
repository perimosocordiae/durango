use std::cell::OnceCell;

use crate::agent::common::*;
use crate::cards::{Card, CardAction};
use crate::data::{BonusToken, Node, Terrain};
use crate::game::{ActionOutcome, DrawAction, GameState, PlayerAction};
use crate::player::Player;

trait GameScorer {
    fn score_game_state(&self, game: &GameState) -> f64;
}

pub(super) struct StaticDistanceTurnPlanner {
    // Score single-hex distances as node.cost^cost_exponent.
    pub cost_exponent: i32,
    dists: OnceCell<Vec<f64>>,
}
impl StaticDistanceTurnPlanner {
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
            game.graph.distances_to_finish(&game.map, 0, |node: &Node| {
                (node.cost as f64).powi(self.cost_exponent)
            })
        })
    }
}
impl Agent for StaticDistanceTurnPlanner {
    fn choose_action(&self, game: &GameState) -> PlayerAction {
        let (best, num_sims) = find_best_action(self, game, 0);
        if num_sims >= 10000 {
            println!("Sims = {num_sims}\t Score = {}", best.score);
        }
        best.action
    }
}
impl GameScorer for StaticDistanceTurnPlanner {
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
}

fn score_player_cards(player: &Player) -> f64 {
    let mut score = 0.0;
    for (card, count) in player.all_cards() {
        score += score_card(card) * (count as f64);
    }
    score
}

fn score_card(card: &Card) -> f64 {
    match card.action {
        None => card.movement.iter().sum::<u8>() as f64,
        Some(CardAction::FreeMove) => 5.0,
        Some(CardAction::Draw(n)) => 2.0 * (n as f64),
        Some(CardAction::DrawAndTrash(n)) => 3.0 * (n as f64),
        Some(CardAction::FreeBuy) => 4.0,
    }
}

#[derive(Default)]
pub(super) struct DynamicCostTurnPlanner {}
impl Agent for DynamicCostTurnPlanner {
    fn choose_action(&self, game: &GameState) -> PlayerAction {
        let (best, num_sims) = find_best_action(self, game, 0);
        if num_sims >= 10000 {
            println!("Sims = {num_sims}\t Score = {}", best.score);
        }
        best.action
    }
}
impl GameScorer for DynamicCostTurnPlanner {
    fn score_game_state(&self, game: &GameState) -> f64 {
        let me = game.curr_player();
        let my_idx = game.map.node_idx(me.position).unwrap();
        let my_board_idx = game.map.node_at_idx(my_idx).unwrap().board_idx;
        let my_cards = me.all_cards();
        let dists = game.graph.distances_to_finish(
            &game.map,
            my_board_idx,
            |node: &Node| 1.0 - traversability(node, &my_cards).ln(),
        );
        let num_tokens = me.tokens.len();
        let num_barriers = me.broken_barriers.len();
        // No need to score cards here since traversability already factors them in.
        (num_tokens as f64) * 10.0
            + (num_barriers as f64) * 100.0
            + dists[my_idx] * -1000.0
    }
}

// Compute the likelihood of being able to traverse this node,
// given the player's cards.
fn traversability(node: &Node, player_cards: &[(&Card, usize)]) -> f64 {
    let mut num_can_traverse = 0;
    let mut total_cards = 0;
    for &(card, count) in player_cards {
        match card.action {
            None => {
                if can_traverse(node, card) {
                    num_can_traverse += count;
                }
                total_cards += count;
            }
            Some(CardAction::FreeMove) => {
                // Free move cards can always traverse.
                num_can_traverse += count;
                total_cards += count;
            }
            Some(_) => {
                // Other actions don't help with movement, but we can make them
                // valuable to the agent by excluding them from the total.
            }
        }
    }
    num_can_traverse as f64 / (total_cards as f64).max(1.0)
}

fn can_traverse(node: &Node, card: &Card) -> bool {
    match node.terrain {
        Terrain::Invalid | Terrain::Cave => false,
        Terrain::Swamp | Terrain::Village => true,
        Terrain::Jungle => card.movement[0] >= node.cost,
        Terrain::Desert => card.movement[1] >= node.cost,
        Terrain::Water => card.movement[2] >= node.cost,
    }
}

struct ActionScore {
    action: PlayerAction,
    score: f64,
}
const MAX_DEPTH: usize = 5;
fn find_best_action(
    agent: &impl GameScorer,
    game: &GameState,
    depth: usize,
) -> (ActionScore, usize) {
    // Hack: to avoid the possibility of an infinite loop of drawing cards,
    // only consider buy/move/trash actions if no draw actions are possible.
    // This also means the agent can't cheat by looking ahead in the deck.
    if let Some(draw) = valid_draw_actions(game).into_iter().next() {
        let action = PlayerAction::Draw(draw);
        return (ActionScore { action, score: 0.0 }, 0);
    }

    let num_cards = game.curr_player().hand.len();
    let mut best = ActionScore {
        action: if num_cards == 0 {
            PlayerAction::FinishTurn
        } else {
            PlayerAction::Discard((0..num_cards).collect())
        },
        score: agent.score_game_state(game),
    };
    if depth >= MAX_DEPTH {
        return (best, 0);
    }

    let mut num_sims = 0;
    for action in all_actions(game) {
        let mut simulated_game = game.clone();
        let outcome = simulated_game.process_action(&action).expect(&format!(
            "Simulation failed for move: {action:?}\nwith tokens: {:?}",
            simulated_game.curr_player().tokens
        ));
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
        let (res, ct) = find_best_action(agent, &simulated_game, depth + 1);
        num_sims += ct;
        if res.score > best.score {
            best = ActionScore {
                action,
                score: res.score,
            };
        }
    }
    // Special case: if we'd simply discard 2+ cards, try using ReplaceHand.
    // We don't simulate this because it would "cheat" by looking at the deck.
    if num_cards > 1
        && matches!(best.action, PlayerAction::Discard(_))
        && let Some(idx) = game
            .curr_player()
            .tokens
            .iter()
            .position(|t| matches!(t, BonusToken::ReplaceHand))
    {
        best.action = PlayerAction::Draw(DrawAction {
            card: None,
            token: Some(idx),
        });
    }
    (best, num_sims)
}

// Get all valid actions for the current player, excluding draw-card actions.
fn all_actions(game: &GameState) -> Vec<PlayerAction> {
    let mut actions = Vec::new();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_choose_action() {
        let game = GameState::new(2, "first", &mut rand::rng()).unwrap();
        let agent = StaticDistanceTurnPlanner::new(0);
        let action = agent.choose_action(&game);
        println!("Chosen action: {:?}", action);
    }
}
