use crate::data::{HexDirection, Terrain};
use crate::game::{GameState, MoveAction, PlayerAction};
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
                Terrain::Swamp | Terrain::Village => {
                    // TODO: generate all length-cost combinations of cards.
                    if me.hand.len() >= node.cost as usize {
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
        // TODO: buy cards
        PlayerAction::Discard((0..me.hand.len()).collect())
    }
}
