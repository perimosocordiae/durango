use crate::data::Node;
use rand::prelude::SliceRandom;
use serde::{Deserialize, Serialize};

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
        let hand = deck.split_off(4);
        Self {
            position,
            deck,
            hand,
            played: Vec::new(),
            discard: Vec::new(),
        }
    }
}

impl GameState {
    fn new(num_players: usize, rng: &mut impl rand::Rng) -> Self {
        Self {
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
