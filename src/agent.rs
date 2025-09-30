use std::cell::OnceCell;
use std::collections::VecDeque;

use crate::cards::{Card, CardAction};
use crate::data::{HexDirection, Terrain};
use crate::game::{
    BuyCardAction, BuyIndex, DrawAction, GameState, MoveAction, PlayerAction,
};
use rand::Rng;

pub trait Agent {
    fn choose_action(&self, game: &GameState) -> PlayerAction;
}

pub fn create_agent(difficulty: usize) -> Box<dyn Agent + Send> {
    match difficulty {
        // Random (valid) actions.
        0 => Box::<RandomAgent>::default(),
        // Very simple heuristics.
        _ => Box::<GreedyAgent>::default(),
    }
}

fn valid_move_actions(game: &GameState) -> Vec<MoveAction> {
    let mut valid_moves = Vec::new();
    let me = game.curr_player();
    for (dir, pos, node) in game.map.neighbors_of(me.position) {
        if game.is_occupied(pos) {
            continue;
        }
        match node.terrain {
            Terrain::Invalid => continue,
            // Avoid caves for now because they're not implemented yet.
            Terrain::Cave => continue,
            Terrain::Jungle => {
                for (i, card) in me.hand.iter().enumerate() {
                    if card.movement[0] >= node.cost {
                        valid_moves.push(MoveAction {
                            cards: vec![i],
                            path: vec![dir],
                        });
                    }
                }
            }
            Terrain::Desert => {
                for (i, card) in me.hand.iter().enumerate() {
                    if card.movement[1] >= node.cost {
                        valid_moves.push(MoveAction {
                            cards: vec![i],
                            path: vec![dir],
                        });
                    }
                }
            }
            Terrain::Water => {
                for (i, card) in me.hand.iter().enumerate() {
                    if card.movement[2] >= node.cost {
                        valid_moves.push(MoveAction {
                            cards: vec![i],
                            path: vec![dir],
                        });
                    }
                }
            }
            Terrain::Swamp => {
                // TODO: generate all length-cost combinations of cards.
                if me.hand.len() >= node.cost as usize {
                    valid_moves.push(MoveAction {
                        cards: (0..node.cost as usize).collect(),
                        path: vec![dir],
                    });
                }
            }
            Terrain::Village => {
                // TODO: generate all length-cost combinations of cards.
                if me.num_cards() > 4 && me.hand.len() >= node.cost as usize {
                    valid_moves.push(MoveAction {
                        cards: (0..node.cost as usize).collect(),
                        path: vec![dir],
                    });
                }
            }
        }
    }
    valid_moves
}

fn valid_buy_actions(game: &GameState) -> Vec<BuyCardAction> {
    let hand = &game.curr_player().hand;
    // Check for FreeBuy cards first.
    for (i, c) in hand.iter().enumerate() {
        if let Some(CardAction::FreeBuy) = c.action {
            // Can buy any card for free, so just return all possible buys.
            let mut buys: Vec<BuyCardAction> = game
                .shop
                .iter()
                .enumerate()
                .map(|(j, _)| BuyCardAction {
                    cards: vec![i],
                    index: BuyIndex::Shop(j),
                })
                .collect();
            buys.extend(game.storage.iter().enumerate().map(|(j, _)| {
                BuyCardAction {
                    cards: vec![i],
                    index: BuyIndex::Storage(j),
                }
            }));
            return buys;
        }
    }
    // Otherwise, buy cards normally.
    if !game.curr_player().can_buy {
        return vec![];
    }
    let cash = hand.iter().map(|c| c.gold_value()).sum();
    let mut buys: Vec<BuyCardAction> = game
        .shop
        .iter()
        .enumerate()
        .filter(|(_, c)| c.cost <= cash)
        .map(|(i, _)| BuyCardAction {
            cards: (0..hand.len()).collect(),
            index: BuyIndex::Shop(i),
        })
        .collect();
    if game.has_open_shop() {
        buys.extend(
            game.storage
                .iter()
                .enumerate()
                .filter(|(_, c)| c.cost <= cash)
                .map(|(i, _)| BuyCardAction {
                    cards: (0..hand.len()).collect(),
                    index: BuyIndex::Storage(i),
                }),
        );
    }
    buys
}

fn valid_draw_actions(game: &GameState) -> Vec<DrawAction> {
    game.curr_player()
        .hand
        .iter()
        .enumerate()
        .filter_map(|(i, c)| match c.action {
            Some(CardAction::Draw(_)) | Some(CardAction::DrawAndTrash(_)) => {
                Some(DrawAction { card: i })
            }
            _ => None,
        })
        .collect()
}

#[derive(Default)]
struct RandomAgent {}
impl Agent for RandomAgent {
    fn choose_action(&self, game: &GameState) -> PlayerAction {
        let mut rng = rand::rng();
        let me = game.curr_player();
        if me.hand.is_empty() {
            return PlayerAction::FinishTurn;
        }
        let mut valid_draws = valid_draw_actions(game);
        if !valid_draws.is_empty() {
            let idx = rng.random_range(0..valid_draws.len());
            return PlayerAction::Draw(valid_draws.swap_remove(idx));
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
        PlayerAction::Discard((0..me.hand.len()).collect())
    }
}

struct MoveCandidate {
    node_idx: usize,
    action: MoveAction,
}

fn all_free_moves(
    game: &GameState,
    card_idx: usize,
    my_idx: usize,
) -> impl Iterator<Item = MoveCandidate> {
    game.map
        .neighbor_indices(my_idx)
        .filter_map(move |(nbr_idx, dir)| {
            let (pos, node) = game.map.nodes.get(nbr_idx)?;
            if game.is_occupied(*pos) {
                return None;
            }
            match node.terrain {
                Terrain::Invalid | Terrain::Cave => None,
                _ => Some(MoveCandidate {
                    node_idx: nbr_idx,
                    action: MoveAction {
                        cards: vec![card_idx],
                        path: vec![dir],
                    },
                }),
            }
        })
}

fn all_moves_for_card<'a>(
    card: &'a Card,
    card_idx: usize,
    game: &'a GameState,
    my_idx: usize,
) -> Box<dyn Iterator<Item = MoveCandidate> + 'a> {
    // Check for free moves first.
    if let Some(CardAction::FreeMove) = card.action {
        return Box::new(all_free_moves(game, card_idx, my_idx));
    }
    // Otherwise, we need to consider terrain and cost.
    let max_move = *card.movement.iter().max().unwrap();
    struct QueueElem {
        idx: usize,
        path: Vec<HexDirection>,
        cost: [u8; 3],
    }
    let mut queue = VecDeque::new();
    queue.push_back(QueueElem {
        idx: my_idx,
        path: Vec::new(),
        cost: [0u8; 3],
    });
    // Track paths for all seen hexes.
    let mut seen = vec![(my_idx, Vec::<HexDirection>::new())];
    while let Some(QueueElem { idx, path, cost }) = queue.pop_front() {
        if path.len() >= max_move as usize {
            continue;
        }
        for (nbr_idx, dir) in game.map.neighbor_indices(idx) {
            let (pos, node) = game.map.nodes[nbr_idx];
            if node.cost > max_move {
                continue;
            }
            let terrain_idx = match node.terrain {
                Terrain::Jungle => 0,
                Terrain::Desert => 1,
                Terrain::Water => 2,
                _ => {
                    continue;
                }
            };
            if game.is_occupied(pos) {
                continue;
            }
            if seen.iter().any(|(i, _)| *i == nbr_idx) {
                continue;
            }
            let mut new_cost = cost;
            new_cost[terrain_idx] += node.cost;
            if new_cost[terrain_idx] > card.movement[terrain_idx] {
                continue;
            }
            if new_cost.iter().filter(|&&c| c > 0).count() != 1 {
                continue;
            }
            let mut new_path = path.clone();
            new_path.push(dir);
            seen.push((nbr_idx, new_path.clone()));
            queue.push_back(QueueElem {
                idx: nbr_idx,
                path: new_path,
                cost: new_cost,
            });
        }
    }
    // Drop the first seen entry because it's a null move.
    Box::new(seen.into_iter().skip(1).map(move |(node_idx, path)| {
        MoveCandidate {
            node_idx,
            action: MoveAction {
                cards: vec![card_idx],
                path,
            },
        }
    }))
}

fn best_move_for_node(
    node_idx: usize,
    dir: HexDirection,
    game: &GameState,
    hand: &[Card],
) -> Option<MoveCandidate> {
    let (pos, node) = game.map.nodes[node_idx];
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
        action: MoveAction {
            cards: card_indices,
            path: vec![dir],
        },
    })
}

#[derive(Default)]
struct GreedyAgent {
    hex_dists: OnceCell<Vec<i32>>,
}
impl Agent for GreedyAgent {
    fn choose_action(&self, game: &GameState) -> PlayerAction {
        let mut rng = rand::rng();
        let me = game.curr_player();
        if me.hand.is_empty() {
            return PlayerAction::FinishTurn;
        }

        // Play any draw cards.
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

        // Try to move as close to the finish as possible.
        let dists = self.hex_dists.get_or_init(|| game.map.hexes_to_finish());
        let my_idx = game.map.node_idx(me.position).unwrap();
        // Look at all one-card moves first.
        let moves = me
            .hand
            .iter()
            .enumerate()
            .flat_map(|(i, c)| all_moves_for_card(c, i, game, my_idx));
        // Now consider any multi-card moves (single-tile only).
        let moves = moves.chain(game.map.neighbor_indices(my_idx).filter_map(
            |(nbr_idx, dir)| best_move_for_node(nbr_idx, dir, game, &me.hand),
        ));
        // TODO: score moves by some heuristic function instead of just distance
        // to the finish. Account for value of cards used, etc.
        if let Some(cand) = moves.min_by_key(|cand| dists[cand.node_idx])
            && dists[cand.node_idx] < dists[my_idx]
        {
            return PlayerAction::Move(cand.action);
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
            let mut buys: Vec<BuyCardAction> = game
                .shop
                .iter()
                .enumerate()
                .filter(|(_, c)| c.cost == max_cost)
                .map(|(i, _)| BuyCardAction {
                    cards: (0..hand_len).collect(),
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
                            index: BuyIndex::Storage(i),
                        }),
                );
            }
            let idx = rng.random_range(0..buys.len());
            return PlayerAction::BuyCard(buys.swap_remove(idx));
        }

        // Nothing else to do, discard all cards.
        PlayerAction::Discard((0..hand_len).collect())
    }
}
