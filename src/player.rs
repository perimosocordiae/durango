use crate::cards::Card;
use crate::data::AxialCoord;
use rand::prelude::SliceRandom;
use serde::{Deserialize, Serialize};

const HAND_SIZE: usize = 4;

#[derive(Serialize, Deserialize, Clone)]
pub struct Player {
    pub position: AxialCoord,
    deck: Vec<Card>,
    pub hand: Vec<Card>,
    pub(crate) played: Vec<Card>,
    pub(crate) discard: Vec<Card>,
    pub(crate) trashes: usize,
    pub can_buy: bool,
}

fn rev_sorted(xs: &[usize]) -> Vec<usize> {
    let mut result = xs.to_vec();
    result.sort_unstable_by(|a, b| b.cmp(a));
    result
}

impl Player {
    pub(crate) fn new(position: AxialCoord, rng: &mut impl rand::Rng) -> Self {
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
            trashes: 0,
            can_buy: true,
        }
    }
    /// Move specified `cards` from self.hand into self.played.
    pub(crate) fn mark_played(&mut self, cards: &[usize]) {
        for i in rev_sorted(cards) {
            self.played.push(self.hand.swap_remove(i));
        }
    }
    /// Move specified `cards` from self.hand directly into self.discard.
    pub(crate) fn discard_cards(&mut self, cards: &[usize]) {
        for i in rev_sorted(cards) {
            self.discard.push(self.hand.swap_remove(i));
        }
    }
    /// Remove specified `cards` from self.hand permanently.
    pub(crate) fn trash_cards(&mut self, cards: &[usize]) {
        for i in rev_sorted(cards) {
            self.hand.swap_remove(i);
        }
    }
    /// Fill hand from the deck, adding shuffled cards from the discard if needed.
    pub(crate) fn fill_hand(
        &mut self,
        hand_size: usize,
        rng: &mut impl rand::Rng,
    ) {
        while self.hand.len() < hand_size {
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
    /// Clean up after the turn is over.
    pub(crate) fn finish_turn(&mut self, rng: &mut impl rand::Rng) {
        // Discard all played cards.
        self.discard.append(&mut self.played);
        // Refill the hand for the next turn.
        self.fill_hand(HAND_SIZE, rng);
        // Reset per-turn state.
        self.trashes = 0;
        self.can_buy = true;
    }

    /// Total cards belonging to the player.
    pub fn num_cards(&self) -> usize {
        self.hand.len()
            + self.deck.len()
            + self.played.len()
            + self.discard.len()
    }

    pub fn debug_str(&self, idx: usize) -> String {
        format!(
            "P{idx}{:?}: hand={:?}, deck={}, played={}, discard={}, can_buy={}",
            self.position,
            &self.hand,
            self.deck.len(),
            self.played.len(),
            self.discard.len(),
            self.can_buy
        )
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
        let p = Player::new(AxialCoord { q: 3, r: -2 }, &mut rand::rng());
        assert_eq!(p.position, AxialCoord { q: 3, r: -2 });
        assert_eq!(p.hand.len(), HAND_SIZE);
        assert_eq!(p.deck.len(), 4);
        assert_eq!(p.played.len(), 0);
        assert_eq!(p.discard.len(), 0);
        assert_eq!(p.trashes, 0);
        assert!(p.can_buy);
        assert_eq!(p.num_cards(), 8);
    }
}
