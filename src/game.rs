use crate::data::{easy_1, load_nodes, HexDirection, Node, Terrain};
use rand::prelude::SliceRandom;
use serde::{Deserialize, Serialize};

const HAND_SIZE: usize = 4;
const MOVE_TYPES: [&str; 3] = ["jungle", "desert", "water"];

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
pub struct GameState {
    nodes: Vec<Node>,
    players: Vec<Player>,
    shop: Vec<BuyableCard>,
    storage: Vec<BuyableCard>,
    curr_player_idx: usize,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum BuyIndex {
    Shop(usize),
    Storage(usize),
}

#[derive(Serialize, Deserialize)]
pub struct BuyCardAction {
    cards: Vec<usize>,
    index: BuyIndex,
}

#[derive(Serialize, Deserialize)]
pub struct MoveAction {
    cards: Vec<usize>,
    path: Vec<HexDirection>,
}

#[derive(Serialize, Deserialize)]
pub enum PlayerAction {
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

fn move_cards(cards: &[usize], src: &mut Vec<Card>, dest: &mut Vec<Card>) {
    let mut rev_sorted_cards = cards.to_vec();
    rev_sorted_cards.sort_unstable_by(|a, b| b.cmp(a));
    for i in rev_sorted_cards {
        dest.push(src.swap_remove(i));
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
    /// Move specified `cards` from self.hand into self.played.
    fn mark_played(&mut self, cards: &[usize]) {
        move_cards(cards, &mut self.hand, &mut self.played);
    }
    /// Move specified `cards` from self.hand into self.discard.
    fn discard_cards(&mut self, cards: &[usize]) {
        move_cards(cards, &mut self.hand, &mut self.discard);
    }
    /// Remove specified `cards` from self.hand permanently.
    fn trash_cards(&mut self, cards: &[usize]) {
        let mut tmp = Vec::new();
        move_cards(cards, &mut self.hand, &mut tmp);
    }
    /// Clean up after the turn is over.
    fn finish_turn(&mut self, rng: &mut impl rand::Rng) {
        // Trash any played cards marked as single-use.
        for i in (0..self.played.len()).rev() {
            if self.played[i].single_use {
                self.played.swap_remove(i);
            }
        }
        // Discard any remaining played cards.
        self.discard.append(&mut self.played);
        // Refill the hand from the deck.
        while self.hand.len() < HAND_SIZE {
            if self.deck.is_empty() && !self.discard.is_empty() {
                // Shuffle the discard pile into the deck.
                self.deck.append(&mut self.discard);
                self.deck.shuffle(rng);
            }
            if let Some(card) = self.deck.pop() {
                self.hand.push(card);
            } else {
                break;
            }
        }
    }
}

impl GameState {
    pub fn new(num_players: usize, rng: &mut impl rand::Rng) -> Self {
        Self {
            nodes: load_nodes(&easy_1()),
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

    pub fn process_action(&mut self, action: &PlayerAction) -> Result<(), String> {
        match action {
            PlayerAction::BuyCard(buy) => {
                {
                    let bucks: u8 = buy
                        .cards
                        .iter()
                        .map(|i| self.curr_player().deck[*i].gold_value())
                        .sum();
                    let card = match buy.index {
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
                let mut card_cost = 0;
                let mut visited_cave = false;
                for dir in &mv.path {
                    let mut next_idx = self.nodes[idx].neighbors[*dir as usize];
                    let next_node = &self.nodes[next_idx];
                    match next_node.terrain {
                        Terrain::Jungle => move_cost[0] += next_node.cost,
                        Terrain::Desert => move_cost[1] += next_node.cost,
                        Terrain::Water => move_cost[2] += next_node.cost,
                        Terrain::Invalid => return Err("Invalid move".to_string()),
                        Terrain::Cave => {
                            visited_cave = true;
                            next_idx = idx;
                        }
                        Terrain::Swamp => card_cost += next_node.cost,
                        Terrain::Village => card_cost += next_node.cost,
                    }
                    idx = next_idx;
                }

                // Validate cave visit (doesn't update player position or cards).
                if visited_cave {
                    if mv.path.len() != 1 {
                        return Err("Can only step once to visit a cave".to_string());
                    }
                    if !mv.cards.is_empty() {
                        return Err("Cannot use cards to visit a cave".to_string());
                    }
                    todo!("Implement cave bonus");
                    // return Ok(());
                }
                // Validate discarding / trashing cards.
                if card_cost > 0 {
                    if mv.path.len() != 1 {
                        return Err("Only one step allowed".to_string());
                    }
                    if mv.cards.len() != card_cost as usize {
                        return Err(format!(
                            "Need {} cards to discard/trash, but got {}",
                            card_cost,
                            mv.cards.len()
                        ));
                    }
                } else {
                    // Validate normal movement.
                    if mv.cards.len() != 1 {
                        return Err("Must use a single card to move".to_string());
                    }
                    let total_cost: u8 = move_cost.iter().sum();
                    let max_cost: u8 = *move_cost.iter().max().unwrap();
                    if total_cost != max_cost {
                        return Err("Path must contain a single movement type".to_string());
                    }
                    let card = &self.curr_player().hand[mv.cards[0]];
                    for (i, move_type) in MOVE_TYPES.iter().enumerate() {
                        if move_cost[i] > card.movement[i] {
                            return Err(format!(
                                "Need {} {} movement, but card only has {}",
                                move_cost[i], move_type, card.movement[i]
                            ));
                        }
                    }
                }

                // Update the player's position and cards.
                let player = &mut self.players[self.curr_player_idx];
                player.position = idx;
                if card_cost > 0 {
                    if matches!(self.nodes[idx].terrain, Terrain::Village) {
                        player.trash_cards(&mv.cards);
                    } else {
                        player.discard_cards(&mv.cards);
                    }
                } else {
                    player.mark_played(&mv.cards);
                }
            }
            PlayerAction::Discard(cards) => {
                self.players[self.curr_player_idx].discard_cards(cards);
            }
            PlayerAction::FinishTurn => {
                self.players[self.curr_player_idx].finish_turn(&mut rand::thread_rng());
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
