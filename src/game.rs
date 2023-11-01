use crate::data::{HexDirection, Node, Terrain};
use rand::prelude::SliceRandom;
use serde::{Deserialize, Serialize};

const HAND_SIZE: usize = 4;

#[derive(Serialize, Deserialize)]
enum CardAction {
    FreeBuy,
    FreeMove,
    Draw(usize),
    DrawAndTrash(usize),
}

#[derive(Serialize, Deserialize)]
struct BuyableCard {
    cost: u8,
    movement: [u8; 3],
    single_use: bool,
    action: Option<CardAction>,
    quantity: u8,
}

#[derive(Serialize, Deserialize)]
struct Card {
    // [Jungle, Desert, Water]
    movement: [u8; 3],
    single_use: bool,
}

#[derive(Serialize, Deserialize)]
struct Player {
    position: usize,
    deck: Vec<Card>,
    hand: Vec<Card>,
    played: Vec<Card>,
    discard: Vec<Card>,
}

#[derive(Serialize, Deserialize)]
struct GameState {
    nodes: Vec<Node>,
    players: Vec<Player>,
    shop: Vec<BuyableCard>,
    storage: Vec<BuyableCard>,
    curr_player_idx: usize,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
enum BuyIndex {
    Shop(usize),
    Storage(usize),
}

#[derive(Serialize, Deserialize)]
struct BuyCardAction {
    cards: Vec<usize>,
    index: BuyIndex,
}

#[derive(Serialize, Deserialize)]
struct MoveAction {
    cards: Vec<usize>,
    path: Vec<HexDirection>,
}

#[derive(Serialize, Deserialize)]
enum PlayerAction {
    BuyCard(BuyCardAction),
    Move(MoveAction),
    Discard(Vec<usize>),
    FinishTurn,
}

//////////////////////////
// Implementations      //
//////////////////////////

impl BuyableCard {
    fn regular(cost: u8, movement: [u8; 3]) -> Self {
        Self {
            cost,
            movement,
            single_use: false,
            action: None,
            quantity: 3,
        }
    }
    fn single_use(cost: u8, movement: [u8; 3]) -> Self {
        Self {
            cost,
            movement,
            single_use: true,
            action: None,
            quantity: 3,
        }
    }
    fn action(cost: u8, action: CardAction, single_use: bool) -> Self {
        Self {
            cost,
            movement: [0, 0, 0],
            single_use,
            action: Some(action),
            quantity: 3,
        }
    }
}

impl Card {
    fn gold_value(&self) -> u8 {
        1.max(2 * self.movement[1])
    }
    fn explorer() -> Self {
        Self {
            movement: [1, 0, 0],
            single_use: false,
        }
    }
    fn traveler() -> Self {
        Self {
            movement: [0, 1, 0],
            single_use: false,
        }
    }
    fn sailor() -> Self {
        Self {
            movement: [0, 0, 1],
            single_use: false,
        }
    }
}

impl Player {
    fn new(position: usize, rng: &mut impl rand::Rng) -> Self {
        let mut deck = vec![
            Card::explorer(),
            Card::explorer(),
            Card::explorer(),
            Card::traveler(),
            Card::traveler(),
            Card::traveler(),
            Card::traveler(),
            Card::sailor(),
        ];
        deck.shuffle(rng);
        let hand = deck.split_off(HAND_SIZE);
        Self {
            position,
            deck,
            hand,
            played: Vec::new(),
            discard: Vec::new(),
        }
    }
    fn mark_played(&mut self, cards: &[usize]) {
        for i in cards {
            self.played.push(self.hand.swap_remove(*i));
        }
    }
}

impl GameState {
    fn new(num_players: usize, rng: &mut impl rand::Rng) -> Self {
        Self {
            // TODO: Load the board here.
            nodes: Vec::new(),
            players: (0..num_players).map(|i| Player::new(i, rng)).collect(),
            shop: vec![
                // Scout
                BuyableCard::regular(2, [2, 0, 0]),
                // Trailblazer
                BuyableCard::regular(6, [3, 0, 0]),
                // Jack of all trades
                BuyableCard::regular(4, [1, 1, 1]),
                // Photographer
                BuyableCard::regular(4, [0, 2, 0]),
                // Treasure chest
                BuyableCard::single_use(6, [0, 4, 0]),
                // Transmitter
                BuyableCard::action(8, CardAction::FreeBuy, true),
            ],
            storage: vec![
                // Journalist
                BuyableCard::regular(6, [0, 3, 0]),
                // Millionaire
                BuyableCard::regular(10, [0, 4, 0]),
                // Captain
                BuyableCard::regular(4, [0, 0, 3]),
                // Pioneer
                BuyableCard::regular(10, [5, 0, 0]),
                // Giant Machete
                BuyableCard::single_use(6, [6, 0, 0]),
                // Adventurer
                BuyableCard::regular(8, [2, 2, 2]),
                // Propeller plane
                BuyableCard::single_use(8, [4, 4, 4]),
                // Compass
                BuyableCard::action(4, CardAction::Draw(3), true),
                // Cartographer
                BuyableCard::action(8, CardAction::Draw(2), false),
                // Scientist
                BuyableCard::action(8, CardAction::DrawAndTrash(1), false),
                // Travel log
                BuyableCard::action(6, CardAction::DrawAndTrash(2), true),
                // Native
                BuyableCard::action(10, CardAction::FreeMove, false),
            ],
            curr_player_idx: 0,
        }
    }

    fn curr_player(&self) -> &Player {
        &self.players[self.curr_player_idx]
    }

    fn process_action(&mut self, action: &PlayerAction) -> Result<(), String> {
        match action {
            PlayerAction::BuyCard(buy) => {
                {
                    let bucks: u8 = buy
                        .cards
                        .iter()
                        .map(|i| self.curr_player().deck[*i].gold_value())
                        .sum();
                    let mut card = match buy.index {
                        BuyIndex::Shop(i) => &mut self.shop[i],
                        BuyIndex::Storage(i) => &mut self.storage[i],
                    };
                    if card.quantity == 0 {
                        return Err("Card is out of stock".to_string());
                    }
                    if bucks < card.cost {
                        return Err(format!(
                            "Not enough gold: have {}, need {}",
                            bucks, card.cost
                        ));
                    }
                    card.quantity -= 1;
                }
                self.players[self.curr_player_idx].mark_played(&buy.cards);
            }
            PlayerAction::Move(mv) => {
                let mut idx = self.curr_player().position;
                let mut move_cost: [u8; 3] = [0, 0, 0];
                for dir in &mv.path {
                    let next_idx = self.nodes[idx].neighbors[*dir as usize];
                    match self.nodes[next_idx].terrain {
                        Terrain::Jungle => move_cost[0] += 1,
                        Terrain::Desert => move_cost[1] += 1,
                        Terrain::Water => move_cost[2] += 1,
                        Terrain::Invalid => return Err("Invalid move".to_string()),
                        Terrain::Cave => todo!(),
                        Terrain::Swamp => todo!(),
                        Terrain::Village => todo!(),
                    }
                    idx = next_idx;
                }
                let mut player = &mut self.players[self.curr_player_idx];
                // TODO: check move_cost against mv.card.movement before updating the player's position.
                player.position = idx;
                player.mark_played(&mv.cards);
            }
            PlayerAction::Discard(cards) => {
                self.players[self.curr_player_idx].mark_played(&cards);
            }
            PlayerAction::FinishTurn => {
                // Move all cards from 'played' pile to 'discard' pile.
                let mut player = &mut self.players[self.curr_player_idx];
                player.discard.append(&mut player.played);
                // Refill the hand from the deck.
                let mut deck = &mut player.deck;
                let mut hand = &mut player.hand;
                while hand.len() < HAND_SIZE {
                    if deck.is_empty() && !player.discard.is_empty() {
                        // Shuffle the discard pile into the deck.
                        deck.append(&mut player.discard);
                        deck.shuffle(&mut rand::thread_rng());
                    }
                    if let Some(card) = deck.pop() {
                        hand.push(card);
                    } else {
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}

//////////////////////////
// Tests                //
//////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialization() {
        let game = GameState::new(4, &mut rand::thread_rng());
        assert_eq!(game.players.len(), 4);
        assert_eq!(game.shop.len(), 6);
        assert_eq!(game.storage.len(), 12);
    }

    #[test]
    fn gold_value() {
        let card = Card {
            movement: [0, 1, 0],
            single_use: false,
        };
        assert_eq!(card.gold_value(), 2);

        let card = Card {
            movement: [0, 0, 1],
            single_use: false,
        };
        assert_eq!(card.gold_value(), 1);

        let card = Card {
            movement: [0, 5, 0],
            single_use: false,
        };
        assert_eq!(card.gold_value(), 10);
    }
}
