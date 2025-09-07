use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum CardAction {
    FreeBuy,
    FreeMove,
    Draw(usize),
    DrawAndTrash(usize),
}

#[derive(Serialize, Deserialize)]
pub struct BuyableCard {
    pub cost: u8,
    pub movement: [u8; 3],
    pub single_use: bool,
    pub action: Option<CardAction>,
    pub quantity: u8,
}

impl BuyableCard {
    pub fn regular(cost: u8, movement: [u8; 3]) -> Self {
        Self {
            cost,
            movement,
            single_use: false,
            action: None,
            quantity: 3,
        }
    }
    pub fn single_use(cost: u8, movement: [u8; 3]) -> Self {
        Self {
            cost,
            movement,
            single_use: true,
            action: None,
            quantity: 3,
        }
    }
    pub fn action(cost: u8, action: CardAction, single_use: bool) -> Self {
        Self {
            cost,
            movement: [0, 0, 0],
            single_use,
            action: Some(action),
            quantity: 3,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Card {
    // [Jungle, Desert, Water]
    pub movement: [u8; 3],
    pub single_use: bool,
    pub action: Option<CardAction>,
}

impl Card {
    pub fn gold_value(&self) -> u8 {
        1.max(2 * self.movement[1])
    }
    pub fn explorer() -> Self {
        Self {
            movement: [1, 0, 0],
            single_use: false,
            action: None,
        }
    }
    pub fn traveler() -> Self {
        Self {
            movement: [0, 1, 0],
            single_use: false,
            action: None,
        }
    }
    pub fn sailor() -> Self {
        Self {
            movement: [0, 0, 1],
            single_use: false,
            action: None,
        }
    }
}
impl std::fmt::Debug for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.movement[0] > 0 {
            write!(f, "J{}", self.movement[0])?;
        }
        if self.movement[1] > 0 {
            write!(f, "D{}", self.movement[1])?;
        }
        if self.movement[2] > 0 {
            write!(f, "J{}", self.movement[2])?
        }
        if self.single_use {
            f.write_str(" (1x)")?;
        }
        if let Some(a) = &self.action {
            write!(f, " {:?}", a)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gold_value() {
        let card = Card {
            movement: [0, 1, 0],
            single_use: false,
            action: None,
        };
        assert_eq!(card.gold_value(), 2);

        let card = Card {
            movement: [0, 0, 1],
            single_use: false,
            action: None,
        };
        assert_eq!(card.gold_value(), 1);

        let card = Card {
            movement: [0, 5, 0],
            single_use: false,
            action: None,
        };
        assert_eq!(card.gold_value(), 10);
    }
}
