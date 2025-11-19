use crate::cards::{Card, CardAction};
use crate::data::{BonusToken, HexDirection, Terrain};
use crate::game::{
    BuyCardAction, BuyIndex, DrawAction, GameState, MoveAction, PlayerAction,
};
use crate::player::Player;
use std::collections::VecDeque;

pub trait Agent {
    fn choose_action(&self, game: &GameState) -> PlayerAction;
}

pub(super) fn can_safely_trash(me: &Player) -> bool {
    me.trashes > 0
        && !me.hand.is_empty()
        && me.num_cards() > 4
        && me.sum_movement().into_iter().min().unwrap() > 1
}

pub(super) fn valid_move_actions(game: &GameState) -> Vec<MoveAction> {
    let me = game.curr_player();
    let my_idx = game.map.node_idx(me.position).unwrap();
    // Get unique cards in hand to avoid duplicate move generation.
    let mut uniq_hand: Vec<(&Card, usize)> =
        me.hand.iter().enumerate().map(|(i, c)| (c, i)).collect();
    uniq_hand.sort_unstable();
    uniq_hand.dedup_by(|a, b| a.0 == b.0);
    // Start with regular card moves.
    let mut valid_moves: Vec<MoveAction> = uniq_hand
        .into_iter()
        .flat_map(|(c, i)| all_moves_for_card(c, i, game, my_idx))
        .map(|cand| cand.action)
        .collect();
    // Also consider any token-only moves.
    valid_moves
        .extend(all_token_only_moves(game, my_idx).map(|cand| cand.action));
    // Next, consider single-step moves (cave, swamp, village).
    let from_board = game.map.node_at_idx(my_idx).unwrap().board_idx as usize;
    for (dir, pos, node) in game.neighbors_of(me.position) {
        if let Some(barrier_idx) =
            game.barrier_index(from_board, node.board_idx as usize)
        {
            let barrier = &game.barriers[barrier_idx];
            // TODO: generate all length-cost combinations of cards.
            if barrier.terrain == Terrain::Swamp
                && me.hand.len() >= barrier.cost as usize
            {
                valid_moves.push(MoveAction::multi_card(
                    (0..barrier.cost as usize).collect(),
                    dir,
                ));
            }
            continue;
        }
        if game.is_occupied(pos) {
            continue;
        }
        match node.terrain {
            Terrain::Cave => {
                if game.can_visit_cave(pos) {
                    valid_moves.push(MoveAction::cave(dir));
                }
            }
            Terrain::Swamp => {
                // TODO: generate all length-cost combinations of cards.
                if me.hand.len() >= node.cost as usize {
                    valid_moves.push(MoveAction::multi_card(
                        (0..node.cost as usize).collect(),
                        dir,
                    ));
                }
            }
            Terrain::Village => {
                // TODO: generate all length-cost combinations of cards.
                if me.num_cards() > 4 && me.hand.len() >= node.cost as usize {
                    valid_moves.push(MoveAction::multi_card(
                        (0..node.cost as usize).collect(),
                        dir,
                    ));
                }
            }
            _ => {}
        }
    }
    valid_moves
}

pub(super) fn valid_buy_actions(game: &GameState) -> Vec<BuyCardAction> {
    let me = game.curr_player();
    // Empty if no DoubleUse token available, otherwise holds the token index.
    let double_use: Vec<usize> = me
        .tokens
        .iter()
        .position(|t| matches!(t, BonusToken::DoubleUse))
        .map(|i| vec![i])
        .unwrap_or_else(Vec::new);
    // Check for FreeBuy cards first.
    for (i, c) in me.hand.iter().enumerate() {
        if let Some(CardAction::FreeBuy) = c.action {
            // Can buy any card for free, so just return all possible buys.
            let mut buys: Vec<BuyCardAction> = game
                .shop
                .iter()
                .enumerate()
                .map(|(j, _)| BuyCardAction {
                    cards: vec![i],
                    tokens: double_use.clone(),
                    index: BuyIndex::Shop(j),
                })
                .collect();
            buys.extend(game.storage.iter().enumerate().map(|(j, _)| {
                BuyCardAction {
                    cards: vec![i],
                    tokens: double_use.clone(),
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
    let hand_size = me.hand.len();
    let cash = me.hand.iter().map(|c| c.gold_value()).sum();
    // Only use the token if we're using a single-use card to pay.
    let double_use = if me.hand.iter().any(|c| c.single_use) {
        double_use
    } else {
        vec![]
    };
    let mut buys: Vec<BuyCardAction> = game
        .shop
        .iter()
        .enumerate()
        .filter(|(_, c)| c.cost <= cash)
        .map(|(i, _)| BuyCardAction {
            cards: (0..hand_size).collect(),
            tokens: double_use.clone(),
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
                    cards: (0..hand_size).collect(),
                    tokens: double_use.clone(),
                    index: BuyIndex::Storage(i),
                }),
        );
    }
    buys
}

pub(super) fn valid_draw_actions(game: &GameState) -> Vec<DrawAction> {
    let me = game.curr_player();
    let double_use = me
        .tokens
        .iter()
        .position(|t| matches!(t, BonusToken::DoubleUse));
    me.hand
        .iter()
        .enumerate()
        .filter_map(|(i, c)| match c.action {
            Some(CardAction::Draw(_)) | Some(CardAction::DrawAndTrash(_)) => {
                Some(DrawAction {
                    card: Some(i),
                    token: if c.single_use { double_use } else { None },
                })
            }
            _ => None,
        })
        .chain(me.tokens.iter().enumerate().filter_map(|(i, t)| match t {
            BonusToken::DrawCard | BonusToken::TrashCard => Some(DrawAction {
                card: None,
                token: Some(i),
            }),
            _ => None,
        }))
        .collect()
}

pub(super) struct MoveCandidate {
    pub node_idx: usize,
    pub action: MoveAction,
    pub num_barriers: usize,
}

enum MoveIndex {
    Card(usize),
    Token(usize),
}

pub(super) fn all_moves_for_card<'a>(
    card: &'a Card,
    card_idx: usize,
    game: &'a GameState,
    my_idx: usize,
) -> Box<dyn Iterator<Item = MoveCandidate> + 'a> {
    // Check for free moves first.
    if let Some(CardAction::FreeMove) = card.action {
        return Box::new(all_free_moves(
            game,
            MoveIndex::Card(card_idx),
            my_idx,
        ));
    }
    // Otherwise, we need to consider terrain and cost.
    Box::new(
        all_moves_helper(&card.movement, game, my_idx)
            .into_iter()
            .map(move |seen| MoveCandidate {
                node_idx: seen.node_idx,
                action: MoveAction::single_card(card_idx, seen.path),
                num_barriers: seen.num_barriers,
            }),
    )
}

pub(super) fn all_token_only_moves(
    game: &GameState,
    my_idx: usize,
) -> impl Iterator<Item = MoveCandidate> {
    let me = game.curr_player();
    let mut iters: Vec<Box<dyn Iterator<Item = MoveCandidate>>> =
        Vec::with_capacity(me.tokens.len());
    for (i, tok) in me.tokens.iter().enumerate() {
        match tok {
            BonusToken::FreeMove => {
                iters.push(Box::new(all_free_moves(
                    game,
                    MoveIndex::Token(i),
                    my_idx,
                )));
            }
            BonusToken::Jungle(_)
            | BonusToken::Desert(_)
            | BonusToken::Water(_) => {
                let movement = token_to_movement(tok);
                iters.push(Box::new(
                    all_moves_helper(&movement, game, my_idx).into_iter().map(
                        move |seen| MoveCandidate {
                            node_idx: seen.node_idx,
                            action: MoveAction::single_token(i, seen.path),
                            num_barriers: seen.num_barriers,
                        },
                    ),
                ));
            }
            _ => {}
        }
    }
    iters.into_iter().flatten()
}

fn token_to_movement(token: &BonusToken) -> [u8; 3] {
    let mut movement = [0u8; 3];
    match token {
        BonusToken::Jungle(v) => movement[0] = *v,
        BonusToken::Desert(v) => movement[1] = *v,
        BonusToken::Water(v) => movement[2] = *v,
        _ => {}
    }
    movement
}

fn all_free_moves(
    game: &GameState,
    move_idx: MoveIndex,
    my_idx: usize,
) -> impl Iterator<Item = MoveCandidate> {
    let curr_board_idx =
        game.map.node_at_idx(my_idx).unwrap().board_idx as usize;
    game.graph
        .neighbor_indices(my_idx)
        .filter_map(move |(nbr_idx, dir)| {
            let pos = game.map.coord_at_idx(nbr_idx)?;
            if game.is_occupied(pos) {
                return None;
            }
            let node = game.map.node_at_idx(nbr_idx)?;
            // Free moves cannot be used to break barriers.
            if game
                .barrier_index(curr_board_idx, node.board_idx as usize)
                .is_some()
            {
                return None;
            }
            match node.terrain {
                Terrain::Invalid | Terrain::Cave => None,
                _ => Some(MoveCandidate {
                    node_idx: nbr_idx,
                    action: match move_idx {
                        MoveIndex::Token(token_idx) => {
                            MoveAction::single_token(token_idx, vec![dir])
                        }
                        MoveIndex::Card(card_idx) => {
                            MoveAction::single_card(card_idx, vec![dir])
                        }
                    },
                    num_barriers: 0,
                }),
            }
        })
}

struct SeenMove {
    node_idx: usize,
    path: Vec<HexDirection>,
    num_barriers: usize,
}

fn all_moves_helper(
    movement: &[u8; 3],
    game: &GameState,
    my_idx: usize,
) -> Vec<SeenMove> {
    let max_move = *movement.iter().max().unwrap();
    struct QueueElem {
        idx: usize,
        path: Vec<HexDirection>,
        cost: [u8; 3],
        barriers: Vec<usize>,
    }
    let mut queue = VecDeque::new();
    queue.push_back(QueueElem {
        idx: my_idx,
        path: Vec::new(),
        cost: [0u8; 3],
        barriers: Vec::new(),
    });
    // Track paths for all seen hexes.
    let mut seen = vec![SeenMove {
        node_idx: my_idx,
        path: Vec::new(),
        num_barriers: 0,
    }];
    while let Some(elem) = queue.pop_front() {
        if elem.path.len() >= max_move as usize {
            continue;
        }
        let board_idx =
            game.map.node_at_idx(elem.idx).unwrap().board_idx as usize;
        for (nbr_idx, dir) in game.graph.neighbor_indices(elem.idx) {
            let node = game.map.node_at_idx(nbr_idx).unwrap();
            let nbr_board_idx = node.board_idx as usize;
            // Check if we're crossing a barrier for the first time.
            if let Some(barrier_idx) =
                game.barrier_index(board_idx, nbr_board_idx)
                && !elem.barriers.contains(&barrier_idx)
            {
                let barrier = &game.barriers[barrier_idx];
                if barrier.cost > max_move {
                    continue;
                }
                let terrain_idx = match barrier.terrain {
                    Terrain::Jungle => 0,
                    Terrain::Desert => 1,
                    Terrain::Water => 2,
                    _ => {
                        continue;
                    }
                };
                let mut new_cost = elem.cost;
                new_cost[terrain_idx] += barrier.cost;
                if new_cost[terrain_idx] > movement[terrain_idx] {
                    continue;
                }
                if new_cost.iter().filter(|&&c| c > 0).count() != 1 {
                    continue;
                }
                let mut new_path = elem.path.clone();
                new_path.push(dir);
                let mut new_barriers = elem.barriers.clone();
                new_barriers.push(barrier_idx);
                seen.push(SeenMove {
                    node_idx: elem.idx,
                    path: new_path.clone(),
                    num_barriers: new_barriers.len(),
                });
                queue.push_back(QueueElem {
                    idx: elem.idx,
                    path: new_path,
                    cost: new_cost,
                    barriers: new_barriers,
                });
            } else {
                let pos = game.map.coord_at_idx(nbr_idx).unwrap();
                if node.cost > max_move
                    || game.is_occupied(pos)
                    || seen.iter().any(|s| s.node_idx == nbr_idx)
                {
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
                let mut new_cost = elem.cost;
                new_cost[terrain_idx] += node.cost;
                if new_cost[terrain_idx] > movement[terrain_idx] {
                    continue;
                }
                if new_cost.iter().filter(|&&c| c > 0).count() != 1 {
                    continue;
                }
                let mut new_path = elem.path.clone();
                new_path.push(dir);
                seen.push(SeenMove {
                    node_idx: nbr_idx,
                    path: new_path.clone(),
                    num_barriers: elem.barriers.len(),
                });
                queue.push_back(QueueElem {
                    idx: nbr_idx,
                    path: new_path,
                    cost: new_cost,
                    barriers: elem.barriers.clone(),
                });
            }
        }
    }
    // Drop the first seen entry because it's a null move.
    seen.split_off(1)
}
