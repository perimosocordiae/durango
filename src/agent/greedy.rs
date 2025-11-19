use crate::agent::common::*;
use crate::cards::{Card, CardAction};
use crate::data::{BonusToken, HexDirection, Terrain};
use crate::game::{
    BuyCardAction, BuyIndex, DrawAction, GameState, MoveAction, PlayerAction,
};
use rand::Rng;

#[derive(Default)]
pub(super) struct GreedyAgent {}
impl Agent for GreedyAgent {
    fn choose_action(&self, game: &GameState) -> PlayerAction {
        let mut rng = rand::rng();
        let me = game.curr_player();
        let my_idx = game.map.node_idx(me.position).unwrap();

        // Check if we can enter a cave.
        for (dir, pos, node) in game.graph.neighbors_of_idx(&game.map, my_idx) {
            if node.terrain == Terrain::Cave && game.can_visit_cave(pos) {
                return PlayerAction::Move(MoveAction::cave(dir));
            }
        }

        // Play any draw cards / tokens.
        if let Some(act) = valid_draw_actions(game).into_iter().next() {
            return PlayerAction::Draw(act);
        }

        // Play any FreeBuy cards, picking the most expensive card possible.
        if me
            .hand
            .iter()
            .any(|c| matches!(c.action, Some(CardAction::FreeBuy)))
        {
            let buys = valid_buy_actions(game);
            if let Some(max_cost) =
                buys.iter().map(|b| game.buyable_card(&b.index).cost).max()
            {
                let mut best_buys: Vec<BuyCardAction> = buys
                    .into_iter()
                    .filter(|b| game.buyable_card(&b.index).cost == max_cost)
                    .collect();
                let idx = rng.random_range(0..best_buys.len());
                return PlayerAction::BuyCard(best_buys.swap_remove(idx));
            }
        }

        // Trash any starter cards, if we have trashes available, and if it's
        // not going to leave us with too few cards.
        if can_safely_trash(me) {
            let idxs = me
                .hand
                .iter()
                .enumerate()
                .filter_map(|(i, c)| {
                    if c.movement.iter().sum::<u8>() == 1 {
                        Some(i)
                    } else {
                        None
                    }
                })
                .take(me.trashes)
                .collect::<Vec<_>>();
            if !idxs.is_empty() {
                return PlayerAction::Trash(idxs);
            }
        }

        // Try to move as close to the finish as possible.
        // Look at all one-card moves first.
        let moves = me
            .hand
            .iter()
            .enumerate()
            .flat_map(|(i, c)| all_moves_for_card(c, i, game, my_idx));
        // Now consider any multi-card moves (single-tile only).
        let my_board_idx =
            game.map.node_at_idx(my_idx).unwrap().board_idx as usize;
        let moves =
            moves.chain(game.graph.neighbor_indices(my_idx).filter_map(
                |(nbr_idx, dir)| {
                    best_move_for_node(
                        nbr_idx,
                        dir,
                        game,
                        &me.hand,
                        my_board_idx,
                    )
                },
            ));
        // Also consider any token-only moves.
        let moves = moves.chain(all_token_only_moves(game, my_idx));
        // TODO: score moves by some heuristic function instead of just distance
        // to the finish. Account for value of cards used, etc.
        let dists = &game.graph.dists;
        let best_move = moves.min_by_key(|cand| {
            dists[cand.node_idx] - (cand.num_barriers * 10) as i32
        });
        if let Some(cand) = &best_move
            && (dists[cand.node_idx] < dists[my_idx] || cand.num_barriers > 0)
        {
            return PlayerAction::Move(best_move.unwrap().action);
        }

        // TODO: allow buying with token, even if we have no cards.
        if me.hand.is_empty() {
            return PlayerAction::FinishTurn;
        }

        // Try to buy the most expensive card possible.
        let hand_len = me.hand.len();
        let cash = me.hand.iter().map(|c| c.gold_value()).sum();
        if me.can_buy
            && let Some(max_cost) = game
                .all_buyable_cards()
                .filter(|c| c.cost <= cash)
                .map(|c| c.cost)
                .max()
        {
            let double_use = if me.hand.iter().any(|c| c.single_use) {
                me.tokens
                    .iter()
                    .position(|t| matches!(t, BonusToken::DoubleUse))
                    .map(|i| vec![i])
                    .unwrap_or_else(Vec::new)
            } else {
                vec![]
            };
            let mut buys: Vec<BuyCardAction> = game
                .shop
                .iter()
                .enumerate()
                .filter(|(_, c)| c.cost == max_cost)
                .map(|(i, _)| BuyCardAction {
                    cards: (0..hand_len).collect(),
                    tokens: double_use.clone(),
                    index: BuyIndex::Shop(i),
                })
                .collect();
            if game.has_open_shop() {
                buys.extend(
                    game.storage
                        .iter()
                        .enumerate()
                        .filter(|(_, c)| c.cost == max_cost)
                        .map(|(i, _)| BuyCardAction {
                            cards: (0..hand_len).collect(),
                            tokens: double_use.clone(),
                            index: BuyIndex::Storage(i),
                        }),
                );
            }
            let idx = rng.random_range(0..buys.len());
            return PlayerAction::BuyCard(buys.swap_remove(idx));
        }

        // If we can replace our hand with new cards, do so.
        if let Some(idx) = me
            .tokens
            .iter()
            .position(|t| matches!(t, BonusToken::ReplaceHand))
        {
            return PlayerAction::Draw(DrawAction {
                card: None,
                token: Some(idx),
            });
        }

        // We're stuck, so try a lateral move if possible.
        // TODO: if we're stuck for more than one turn, allow back-moves too.
        if let Some(cand) = best_move
            && dists[cand.node_idx] == dists[my_idx]
        {
            return PlayerAction::Move(cand.action);
        }

        // Nothing else to do, discard all cards.
        PlayerAction::Discard((0..hand_len).collect())
    }
}

fn best_move_for_node(
    node_idx: usize,
    dir: HexDirection,
    game: &GameState,
    hand: &[Card],
    board_idx: usize,
) -> Option<MoveCandidate> {
    let node = game.map.node_at_idx(node_idx).unwrap();
    // Check if we're breaking a barrier first.
    if let Some(barrier_idx) =
        game.barrier_index(board_idx, node.board_idx as usize)
    {
        let barrier = &game.barriers[barrier_idx];
        // Only consider swamp barriers, as the other types are handled via
        // regular card moves.
        if !matches!(barrier.terrain, Terrain::Swamp) {
            return None;
        }
        if barrier.cost > hand.len() as u8 {
            return None;
        }
        // Pick card indices to discard, ordered by value.
        let mut to_discard = hand.iter().enumerate().collect::<Vec<_>>();
        to_discard.sort_unstable_by_key(|(_, card)| {
            card.movement.iter().max().unwrap()
        });
        to_discard.truncate(barrier.cost as usize);
        return Some(MoveCandidate {
            node_idx,
            action: MoveAction::multi_card(
                to_discard.into_iter().map(|(i, _)| i).collect(),
                dir,
            ),
            num_barriers: 1,
        });
    }
    let pos = game.map.coord_at_idx(node_idx).unwrap();
    if game.is_occupied(pos) {
        return None;
    }
    let mut card_indices: Vec<usize> = match node.terrain {
        Terrain::Swamp => {
            // Pick card indices to discard, ordered by value.
            let mut tmp = hand.iter().enumerate().collect::<Vec<_>>();
            tmp.sort_unstable_by_key(|(_, card)| {
                card.movement.iter().max().unwrap()
            });
            tmp.into_iter().map(|(i, _)| i).collect()
        }
        Terrain::Village => {
            // Pick card indices to trash, only considering basic movement cards.
            // TODO: Ideally we'd have a value function that scores each card
            // in a context-dependent way, but this heuristic is ok for now.
            hand.iter()
                .enumerate()
                .filter_map(|(i, card)| {
                    if card.movement.iter().sum::<u8>() == 1 {
                        Some(i)
                    } else {
                        None
                    }
                })
                .collect()
        }
        _ => {
            return None;
        }
    };
    if card_indices.len() < node.cost.into() {
        return None;
    }
    card_indices.truncate(node.cost.into());
    Some(MoveCandidate {
        node_idx,
        action: MoveAction::multi_card(card_indices, dir),
        num_barriers: 0,
    })
}
