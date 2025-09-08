use crate::data::{HexDirection, Terrain};
use crate::game::{
    BuyCardAction, BuyIndex, GameState, MoveAction, PlayerAction,
};
use rand::Rng;

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

fn valid_move_actions(game: &GameState) -> Vec<MoveAction> {
    let mut valid_moves = Vec::new();
    let me = game.curr_player();
    for dir in HexDirection::all_directions() {
        let neighbor_pos = dir.neighbor_coord(me.position);
        if let Some(node) = game.map.nodes.get(&neighbor_pos) {
            if game.is_occupied(neighbor_pos) {
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
                    if me.num_cards() > 4 && me.hand.len() >= node.cost as usize
                    {
                        valid_moves.push(MoveAction {
                            cards: (0..node.cost as usize).collect(),
                            path: vec![dir],
                        });
                    }
                }
            }
        }
    }
    valid_moves
}

fn valid_buy_actions(game: &GameState) -> Vec<BuyCardAction> {
    let hand = &game.curr_player().hand;
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

#[derive(Default)]
struct RandomAgent {}
impl Agent for RandomAgent {
    fn choose_action(&self, game: &GameState) -> PlayerAction {
        let mut rng = rand::rng();
        let me = game.curr_player();
        if me.hand.is_empty() {
            return PlayerAction::FinishTurn;
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
