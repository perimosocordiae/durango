use crate::cards::{BuyableCard, Card, CardAction};
use crate::data::{
    easy_1, load_nodes, AxialCoord, HexDirection, HexMap, Terrain,
};
use rand::prelude::SliceRandom;
use serde::{Deserialize, Serialize};

const HAND_SIZE: usize = 4;
const MOVE_TYPES: [&str; 3] = ["jungle", "desert", "water"];

#[derive(Serialize, Deserialize)]
pub struct Player {
    pub position: AxialCoord,
    deck: Vec<Card>,
    pub hand: Vec<Card>,
    played: Vec<Card>,
    discard: Vec<Card>,
}

#[derive(Serialize, Deserialize)]
pub struct GameState {
    pub map: HexMap,
    players: Vec<Player>,
    pub shop: Vec<BuyableCard>,
    pub storage: Vec<BuyableCard>,
    curr_player_idx: usize,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum BuyIndex {
    Shop(usize),
    Storage(usize),
}

#[derive(Serialize, Deserialize)]
pub struct BuyCardAction {
    pub cards: Vec<usize>,
    pub index: BuyIndex,
}

#[derive(Serialize, Deserialize)]
pub struct MoveAction {
    pub cards: Vec<usize>,
    pub path: Vec<HexDirection>,
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

fn rev_sorted(xs: &[usize]) -> Vec<usize> {
    let mut result = xs.to_vec();
    result.sort_unstable_by(|a, b| b.cmp(a));
    result
}

impl Player {
    fn new(position: AxialCoord, rng: &mut impl rand::Rng) -> Self {
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
        for i in rev_sorted(cards) {
            self.played.push(self.hand.swap_remove(i));
        }
    }
    /// Remove specified `cards` from self.hand permanently.
    fn trash_cards(&mut self, cards: &[usize]) {
        for i in rev_sorted(cards) {
            self.hand.swap_remove(i);
        }
    }
    /// Clean up after the turn is over.
    fn finish_turn(&mut self, rng: &mut impl rand::Rng) {
        // Trash any played cards marked as single-use.
        for i in (0..self.played.len()).rev() {
            if self.played[i].single_use {
                // TODO: avoid trashing cards if they weren't used for their
                // specified single-use purpose.
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
            map: load_nodes(&easy_1()).unwrap(),
            players: (0..num_players)
                .map(|i| {
                    // TODO: better starting positions
                    let start_pos = AxialCoord { q: i as i32, r: 0 };
                    Player::new(start_pos, rng)
                })
                .collect(),
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

    /// The player whose turn it is.
    pub fn curr_player(&self) -> &Player {
        &self.players[self.curr_player_idx]
    }

    /// Is the specified node occupied by a player other than the current player?
    pub fn is_occupied(&self, pos: AxialCoord) -> bool {
        self.players
            .iter()
            .enumerate()
            .any(|(i, p)| p.position == pos && i != self.curr_player_idx)
    }

    /// Process the specified `action` for the current player.
    pub fn process_action(
        &mut self,
        action: &PlayerAction,
    ) -> Result<(), String> {
        match action {
            PlayerAction::BuyCard(buy) => self.handle_buy(buy)?,
            PlayerAction::Move(mv) => self.handle_move(mv)?,
            PlayerAction::Discard(cards) => {
                self.players[self.curr_player_idx].mark_played(cards);
            }
            PlayerAction::FinishTurn => {
                self.players[self.curr_player_idx]
                    .finish_turn(&mut rand::rng());
            }
        }
        Ok(())
    }

    fn handle_buy(&mut self, buy: &BuyCardAction) -> Result<(), String> {
        {
            let deck = &self.curr_player().deck;
            let bucks: u8 =
                buy.cards.iter().map(|i| deck[*i].gold_value()).sum();
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
        Ok(())
    }

    fn handle_move(&mut self, mv: &MoveAction) -> Result<(), String> {
        if mv.path.is_empty() {
            return Err("Must move at least once".to_string());
        }
        let mut pos = self.curr_player().position;
        let mut move_cost: [u8; 3] = [0, 0, 0];
        let mut card_cost = 0;
        let mut visited_cave = None;
        for dir in &mv.path {
            let mut next_pos = dir.neighbor_coord(pos);
            let next_node = &self.map.nodes[&next_pos];
            match next_node.terrain {
                Terrain::Jungle => move_cost[0] += next_node.cost,
                Terrain::Desert => move_cost[1] += next_node.cost,
                Terrain::Water => move_cost[2] += next_node.cost,
                Terrain::Invalid => return Err("Invalid move".to_string()),
                Terrain::Cave => {
                    visited_cave = Some(next_pos);
                    next_pos = pos;
                }
                Terrain::Swamp => card_cost += next_node.cost,
                Terrain::Village => card_cost += next_node.cost,
            }
            if self.is_occupied(next_pos) {
                return Err("Cannot move to occupied node".to_string());
            }
            pos = next_pos;
        }

        // Validate cave visit (doesn't update player position or cards).
        if let Some(cave_pos) = visited_cave {
            if mv.path.len() != 1 {
                return Err("Can only visit adjacent caves".to_string());
            }
            if !mv.cards.is_empty() {
                return Err("Cannot use cards to visit a cave".to_string());
            }
            return self.give_bonus(cave_pos);
        }

        if mv.cards.is_empty() {
            return Err("Must use cards to move".to_string());
        }
        // Handle cards that provide a free move.
        if matches!(
            self.curr_player().hand[mv.cards[0]].action,
            Some(CardAction::FreeMove)
        ) {
            if mv.path.len() != 1 {
                return Err("Only one step allowed".to_string());
            }
            card_cost = 0;
            move_cost = [0, 0, 0];
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
                return Err(
                    "Path must contain a single movement type".to_string()
                );
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
        player.position = pos;
        if card_cost > 0
            && matches!(self.map.nodes[&pos].terrain, Terrain::Village)
        {
            player.trash_cards(&mv.cards);
        } else {
            player.mark_played(&mv.cards);
        }
        Ok(())
    }

    fn give_bonus(&mut self, pos: AxialCoord) -> Result<(), String> {
        let cave = &self.map.nodes[&pos];
        if !matches!(cave.terrain, Terrain::Cave) {
            return Err("Not a cave".to_string());
        }
        // TODO: check that we have enough bonuses in the cave.
        todo!("Implement cave bonuses")
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
        let game = GameState::new(4, &mut rand::rng());
        assert_eq!(game.players.len(), 4);
        assert_eq!(game.shop.len(), 6);
        assert_eq!(game.storage.len(), 12);
    }
}
