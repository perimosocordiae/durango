use crate::cards::{BuyableCard, CardAction};
use crate::data::{
    self, AxialCoord, Barrier, BonusToken, HexDirection, HexMap, Node, Terrain,
};
use crate::graph::HexGraph;
use crate::player::Player;
use rand::prelude::SliceRandom;
use serde::{Deserialize, Serialize};

const MOVE_TYPES: [&str; 3] = ["jungle", "desert", "water"];

/// Index of a buyable card in the shop or storage.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum BuyIndex {
    Shop(usize),
    Storage(usize),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BuyCardAction {
    pub cards: Vec<usize>,
    pub tokens: Vec<usize>,
    pub index: BuyIndex,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MoveAction {
    pub cards: Vec<usize>,
    pub tokens: Vec<usize>,
    pub path: Vec<HexDirection>,
}
impl MoveAction {
    pub fn single_card(card: usize, path: Vec<HexDirection>) -> Self {
        Self {
            cards: vec![card],
            tokens: vec![],
            path,
        }
    }
    pub fn single_token(token: usize, path: Vec<HexDirection>) -> Self {
        Self {
            cards: vec![],
            tokens: vec![token],
            path,
        }
    }
    pub fn multi_card(cards: Vec<usize>, dir: HexDirection) -> Self {
        Self {
            cards,
            tokens: vec![],
            path: vec![dir],
        }
    }
    pub fn cave(dir: HexDirection) -> Self {
        Self {
            cards: vec![],
            tokens: vec![],
            path: vec![dir],
        }
    }
    pub fn is_free_move(&self, player: &Player) -> bool {
        if self.cards.len() == 1
            && let Some(card) = player.hand.get(self.cards[0])
        {
            return matches!(card.action, Some(CardAction::FreeMove));
        }
        if self.tokens.len() == 1
            && let Some(tok) = player.tokens.get(self.tokens[0])
        {
            return matches!(tok, BonusToken::FreeMove);
        }
        false
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DrawAction {
    pub card: Option<usize>,
    pub token: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum PlayerAction {
    BuyCard(BuyCardAction),
    Move(MoveAction),
    Draw(DrawAction),
    Trash(Vec<usize>),
    Discard(Vec<usize>),
    FinishTurn,
}

/// Result of performing an action via game.process_action().
pub enum ActionOutcome {
    Ok,
    IgnoreMoveIdx(usize),
    GameOver,
}

#[derive(Clone)]
pub struct GameState {
    pub map: HexMap,
    pub graph: HexGraph,
    pub barriers: Vec<Barrier>,
    pub players: Vec<Player>,
    pub shop: Vec<BuyableCard>,
    pub storage: Vec<BuyableCard>,
    bonuses: Vec<(AxialCoord, Vec<BonusToken>)>,
    pub curr_player_idx: usize,
    pub round_idx: usize,
}

impl GameState {
    pub fn new(
        num_players: usize,
        preset: &str,
        rng: &mut impl rand::Rng,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if !(2..=4).contains(&num_players) {
            return Err(
                format!("Invalid number of players: {num_players}").into()
            );
        }
        let map = HexMap::create_named(preset)?;
        let graph = HexGraph::new(&map);
        // Set up the barriers between boards.
        let mut barrier_types = data::ALL_BARRIER_TYPES.to_vec();
        barrier_types.shuffle(rng);
        barrier_types.truncate(map.finish_idx as usize - 1);
        let barriers = barrier_types
            .into_iter()
            .enumerate()
            .map(|(i, (terrain, cost))| Barrier {
                from_board: i,
                to_board: i + 1,
                terrain,
                cost,
                edges: edges_between_boards(&map, &graph, i, i + 1),
            })
            .collect();
        // Determine starting positions for players.
        let max_dist_indices = graph
            .dists
            .iter()
            .enumerate()
            .filter_map(
                |(i, &d)| if d == graph.max_dist { Some(i) } else { None },
            )
            .collect::<Vec<usize>>();
        if max_dist_indices.len() < num_players {
            return Err(format!(
                "Not enough distinct starting positions ({} players, but only {} max-dist positions)",
                num_players,
                max_dist_indices.len()
            ).into());
        }
        let players = max_dist_indices
            .into_iter()
            .take(num_players)
            .map(|start_idx| {
                let start_pos = map.coord_at_idx(start_idx).unwrap();
                Player::new(start_pos, rng)
            })
            .collect();
        // Initialize cave bonuses.
        let mut all_tokens = data::ALL_BONUS_TOKENS.to_vec();
        all_tokens.shuffle(rng);
        let bonuses = map
            .all_nodes()
            .filter_map(|(pos, node)| {
                if node.terrain == Terrain::Cave && all_tokens.len() >= 4 {
                    Some((pos, all_tokens.split_off(all_tokens.len() - 4)))
                } else {
                    None
                }
            })
            .collect();
        Ok(Self {
            map,
            graph,
            barriers,
            players,
            shop: vec![
                // Scout
                BuyableCard::regular(2, [2, 0, 0]),
                // Jack of all trades
                BuyableCard::regular(4, [1, 1, 1]),
                // Photographer
                BuyableCard::regular(4, [0, 2, 0]),
                // Trailblazer
                BuyableCard::regular(6, [3, 0, 0]),
                // Treasure chest
                BuyableCard::single_use(6, [0, 4, 0]),
                // Transmitter
                BuyableCard::action(8, CardAction::FreeBuy, true),
            ],
            storage: vec![
                // Captain
                BuyableCard::regular(4, [0, 0, 3]),
                // Compass
                BuyableCard::action(4, CardAction::Draw(3), true),
                // Journalist
                BuyableCard::regular(6, [0, 3, 0]),
                // Giant Machete
                BuyableCard::single_use(6, [6, 0, 0]),
                // Travel log
                BuyableCard::action(6, CardAction::DrawAndTrash(2), true),
                // Adventurer
                BuyableCard::regular(8, [2, 2, 2]),
                // Propeller plane
                BuyableCard::single_use(8, [4, 4, 4]),
                // Cartographer
                BuyableCard::action(8, CardAction::Draw(2), false),
                // Scientist
                BuyableCard::action(8, CardAction::DrawAndTrash(1), false),
                // Millionaire
                BuyableCard::regular(10, [0, 4, 0]),
                // Pioneer
                BuyableCard::regular(10, [5, 0, 0]),
                // Native
                BuyableCard::action(10, CardAction::FreeMove, false),
            ],
            bonuses,
            curr_player_idx: 0,
            round_idx: 0,
        })
    }

    /// Assemble a minimum game state from its parts.
    pub(crate) fn from_parts(
        map: HexMap,
        players: Vec<Player>,
        round_idx: usize,
    ) -> Self {
        Self {
            graph: HexGraph::new(&map),
            map,
            barriers: vec![],
            players,
            shop: vec![],
            storage: vec![],
            bonuses: vec![],
            curr_player_idx: 0,
            round_idx,
        }
    }

    /// The player whose turn it is.
    pub fn curr_player(&self) -> &Player {
        &self.players[self.curr_player_idx]
    }

    /// How many players are in the game.
    pub fn num_players(&self) -> usize {
        self.players.len()
    }

    /// Positions of all players in the game.
    pub fn player_positions(&self) -> Vec<AxialCoord> {
        self.players.iter().map(|p| p.position).collect()
    }

    /// Positions and counts of all cave bonuses in the game.
    pub fn bonus_counts(&self) -> Vec<(&AxialCoord, usize)> {
        self.bonuses
            .iter()
            .map(|(pos, toks)| (pos, toks.len()))
            .collect()
    }

    /// Is the specified node occupied by a player other than the current player?
    pub fn is_occupied(&self, pos: AxialCoord) -> bool {
        self.players
            .iter()
            .enumerate()
            .any(|(i, p)| p.position == pos && i != self.curr_player_idx)
    }

    /// Which players (if any) are on a finish hex?
    pub fn players_at_finish(&self) -> Vec<usize> {
        self.players
            .iter()
            .enumerate()
            .filter(|(_, p)| self.map.is_finish(p.position))
            .map(|(i, _)| i)
            .collect()
    }

    /// Are any players on a finish hex?
    pub fn any_finished_player(&self) -> bool {
        self.players.iter().any(|p| self.map.is_finish(p.position))
    }

    /// Score each player, for determining who won.
    pub fn player_scores(&self) -> Vec<i32> {
        self.players
            .iter()
            .map(|p| {
                let pos_idx = self.map.node_idx(p.position).unwrap();
                let remaining_dist = self.graph.dists[pos_idx];
                let mut score = self.graph.max_dist - remaining_dist
                    + p.broken_barriers.len() as i32;
                if remaining_dist == 0 {
                    score += 1000; // bonus for finishing
                }
                score
            })
            .collect()
    }

    /// Process the specified `action` for the current player.
    pub fn process_action(
        &mut self,
        action: &PlayerAction,
    ) -> Result<ActionOutcome, String> {
        let mut outcome = ActionOutcome::Ok;
        match action {
            PlayerAction::BuyCard(buy) => self.handle_buy(buy)?,
            PlayerAction::Move(mv) => {
                if let Some(idx) = self.handle_move(mv)? {
                    outcome = ActionOutcome::IgnoreMoveIdx(idx);
                }
            }
            PlayerAction::Draw(draw) => {
                self.handle_draw(draw, &mut rand::rng())?
            }
            PlayerAction::Trash(trash) => self.handle_trash(trash)?,
            PlayerAction::Discard(cards) => {
                self.players[self.curr_player_idx].discard_cards(cards);
            }
            PlayerAction::FinishTurn => {
                self.players[self.curr_player_idx]
                    .finish_turn(&mut rand::rng());
                self.curr_player_idx += 1;
                if self.curr_player_idx == self.players.len() {
                    self.round_idx += 1;
                    self.curr_player_idx = 0;
                    if self.any_finished_player() {
                        return Ok(ActionOutcome::GameOver);
                    }
                }
            }
        }
        Ok(outcome)
    }

    pub fn has_open_shop(&self) -> bool {
        self.shop.len() < 6
    }

    pub fn buyable_card(&self, idx: &BuyIndex) -> &BuyableCard {
        match idx {
            BuyIndex::Shop(i) => &self.shop[*i],
            BuyIndex::Storage(i) => &self.storage[*i],
        }
    }

    pub fn all_buyable_cards(
        &self,
    ) -> Box<dyn Iterator<Item = &BuyableCard> + '_> {
        if self.has_open_shop() {
            Box::new(self.shop.iter().chain(self.storage.iter()))
        } else {
            Box::new(self.shop.iter())
        }
    }

    fn handle_buy(&mut self, buy: &BuyCardAction) -> Result<(), String> {
        let bcard = self.buyable_card(&buy.index);
        if bcard.quantity == 0 {
            return Err("Card is out of stock".into());
        }
        let player = self.curr_player();
        let hand = &player.hand;
        let tokens = &player.tokens;
        let bucks: u8 = buy
            .cards
            .iter()
            .map(|i| hand[*i].gold_value())
            .chain(buy.tokens.iter().map(|i| tokens[*i].gold_value()))
            .sum();
        let mut is_free_buy = false;
        if bucks < bcard.cost {
            // Check if we're trying to use a FreeBuy card.
            if buy.cards.len() == 1
                && matches!(
                    hand[buy.cards[0]].action,
                    Some(CardAction::FreeBuy)
                )
            {
                is_free_buy = true;
            } else {
                return Err(format!(
                    "Not enough gold: have {}, need {}",
                    bucks / 2,
                    bcard.cost / 2
                ));
            }
        }
        if !is_free_buy && !player.can_buy {
            return Err("Can only buy one card per turn".into());
        }
        // Identify any single-use cards used to pay for the purchase. Note that
        // it's possible to use a single-use card as part of the payment without
        // triggering the single-use condition, so we only want to trash it if
        // it's actually providing non-default value (gold or a free buy).
        let single_use_idxs: Vec<usize> = buy
            .cards
            .iter()
            .cloned()
            .filter(|&i| {
                hand[i].single_use && (is_free_buy || hand[i].gold_value() > 0)
            })
            .collect();
        let card = bcard.to_card();
        if is_free_buy {
            // Just take the card without moving from storage to shop.
            match buy.index {
                BuyIndex::Shop(i) => {
                    take_card(&mut self.shop, i);
                }
                BuyIndex::Storage(i) => {
                    take_card(&mut self.storage, i);
                }
            }
        } else {
            // Move the card from storage to shop if needed, then take it.
            let shop_idx = match buy.index {
                BuyIndex::Shop(i) => i,
                BuyIndex::Storage(i) => {
                    if !self.has_open_shop() {
                        return Err(
                            "Cannot buy from storage while shop is full".into(),
                        );
                    }
                    self.shop.push(self.storage.swap_remove(i));
                    self.shop.len() - 1
                }
            };
            take_card(&mut self.shop, shop_idx);
        }
        // Add the newly-bought card to the player's discard pile.
        self.players[self.curr_player_idx].discard.push(card);
        // Discard or trash the cards used to pay for the purchase.
        if single_use_idxs.is_empty() {
            self.players[self.curr_player_idx].mark_played(&buy.cards);
        } else if single_use_idxs.len() == buy.cards.len() {
            // All cards used were single-use, so trash them all.
            self.players[self.curr_player_idx].trash_cards(&buy.cards);
        } else {
            // We have a mix: some cards to trash, some to discard.
            let p = &mut self.players[self.curr_player_idx];
            for i in &buy.cards {
                if !single_use_idxs.contains(i) {
                    p.played.push(p.hand[*i].clone());
                }
            }
            p.hand.clear();
        }
        // Ensure we only buy one card per turn (excluding free buys).
        if !is_free_buy {
            self.players[self.curr_player_idx].can_buy = false;
        }
        // Ensure the shop remains sorted by cost. We only ever remove from
        // storage, so no need to re-sort that.
        self.shop.sort_unstable_by_key(|c| c.cost);
        // Trash any used tokens. Assumes tokens are in sorted order.
        for idx in buy.tokens.iter().rev() {
            self.players[self.curr_player_idx].tokens.swap_remove(*idx);
        }
        Ok(())
    }

    /// Returns an optional index into mv.path to ignore for movement.
    fn handle_move(
        &mut self,
        mv: &MoveAction,
    ) -> Result<Option<usize>, String> {
        if mv.path.is_empty() {
            return Err("Must move at least once".into());
        }
        let tokens = &self.curr_player().tokens;
        let mut pos = self.curr_player().position;
        let mut board_idx = self
            .map
            .node_at(pos)
            .ok_or::<String>("Invalid position".into())?
            .board_idx as usize;
        let mut move_cost: [u8; 3] = [0, 0, 0];
        let mut card_cost = 0;
        let mut broken_barrier = None;
        let mut visited_cave = None;
        let mut ignore_idx = None;
        for (path_idx, dir) in mv.path.iter().enumerate() {
            let mut next_pos = dir.neighbor_coord(pos);
            if let Some(next_node) = self.map.node_at(next_pos) {
                // Check for barriers first, as these act like pseudo-nodes.
                let next_board_idx = next_node.board_idx as usize;
                if let Some(barrier_idx) =
                    self.barrier_index(board_idx, next_board_idx)
                {
                    let bar = &self.barriers[barrier_idx];
                    match bar.terrain {
                        Terrain::Jungle => move_cost[0] += bar.cost,
                        Terrain::Desert => move_cost[1] += bar.cost,
                        Terrain::Water => move_cost[2] += bar.cost,
                        Terrain::Swamp => card_cost += bar.cost,
                        _ => {
                            return Err(format!("Invalid barrier: {bar:?}",));
                        }
                    }
                    broken_barrier = Some(barrier_idx);
                    ignore_idx = Some(path_idx);
                } else {
                    // Regular movement onto the next node.
                    match next_node.terrain {
                        Terrain::Jungle => move_cost[0] += next_node.cost,
                        Terrain::Desert => move_cost[1] += next_node.cost,
                        Terrain::Water => move_cost[2] += next_node.cost,
                        Terrain::Invalid => {
                            return Err(format!(
                                "Cannot move onto invalid terrain {:?}",
                                next_pos
                            ));
                        }
                        Terrain::Cave => {
                            visited_cave = Some(next_pos);
                            ignore_idx = Some(path_idx);
                            next_pos = pos;
                        }
                        Terrain::Swamp => card_cost += next_node.cost,
                        Terrain::Village => card_cost += next_node.cost,
                    }
                    if visited_cave.is_none()
                        && self.is_occupied(next_pos)
                        && !mv
                            .tokens
                            .iter()
                            .any(|&i| matches!(tokens[i], BonusToken::ShareHex))
                    {
                        return Err(format!(
                            "Cannot move to occupied node {:?}",
                            next_pos
                        ));
                    }
                    pos = next_pos;
                }
                board_idx = next_board_idx;
            } else {
                return Err(format!("No node at position {:?}", next_pos));
            }
        }

        // Validate cave visit (doesn't update player position or cards).
        if let Some(cave_pos) = visited_cave {
            if mv.path.len() != 1 {
                return Err("Can only visit adjacent caves".to_string());
            }
            if !mv.cards.is_empty() {
                return Err("Cannot use cards to visit a cave".to_string());
            }
            if self.curr_player().visited_caves.contains(&cave_pos) {
                return Err("Already visited this cave, must move away before returning".into());
            }
            self.give_bonus(cave_pos)?;
            return Ok(ignore_idx);
        }

        // Handle cards/tokens that provide a free move.
        let path_len = mv.path.len();
        if mv.is_free_move(self.curr_player()) {
            if path_len != 1 {
                return Err(format!(
                    "Only one step allowed for free movement, got {path_len}"
                ));
            }
            card_cost = 0;
            move_cost = [0, 0, 0];
            if let Some(barrier_idx) = broken_barrier {
                let barrier = &self.barriers[barrier_idx];
                match barrier.terrain {
                    // Movement-based barriers cannot be broken with a free move.
                    Terrain::Jungle | Terrain::Desert | Terrain::Water => {
                        return Err(
                            "Cannot break barriers with a free move".into()
                        );
                    }
                    // Swamp barriers still require the card cost.
                    Terrain::Swamp => {
                        card_cost = barrier.cost;
                    }
                    _ => {
                        return Err(format!("Invalid barrier: {barrier:?}"));
                    }
                }
            }
        }

        let mut is_single_use = false;
        if card_cost > 0 {
            // Validate discarding / trashing cards.
            if path_len != 1 {
                return Err(format!(
                    "Can only move one step when discarding/trashing cards, got {path_len}",
                ));
            }
            if mv.cards.len() != card_cost as usize {
                return Err(format!(
                    "Need {} cards to discard/trash, but got {}",
                    card_cost,
                    mv.cards.len()
                ));
            }
            // Check that no invalid tokens are being used.
            if mv.tokens.len() > 1
                || (mv.tokens.len() == 1
                    && !matches!(tokens[mv.tokens[0]], BonusToken::ShareHex))
            {
                return Err(
                    "Only the ShareHex token can be used when discarding/trashing cards".into()
                );
            }
        } else {
            // Validate normal movement.
            let total_cost: u8 = move_cost.iter().sum();
            let max_cost: u8 = *move_cost.iter().max().unwrap();
            if total_cost != max_cost {
                return Err(format!(
                    "Path must contain a single movement type, got J={}, D={}, W={}",
                    move_cost[0], move_cost[1], move_cost[2]
                ));
            }
            let hand = &self.curr_player().hand;

            // Card movement.
            if !mv.cards.is_empty() {
                if mv.cards.len() != 1 {
                    return Err(format!(
                        "Must use a single card to move, got {}",
                        mv.cards.len()
                    ));
                }
                let card = &hand[mv.cards[0]];
                // Ensure we have enough movement of the required type.
                if mv
                    .tokens
                    .iter()
                    .any(|&i| matches!(tokens[i], BonusToken::SwapSymbol))
                {
                    let m = *card.movement.iter().max().unwrap();
                    if m < max_cost {
                        return Err(format!(
                            "Need {max_cost}+ movement, but card {card:?} can only move {m}",
                        ));
                    }
                } else {
                    for (i, move_type) in MOVE_TYPES.iter().enumerate() {
                        if move_cost[i] > card.movement[i] {
                            return Err(format!(
                                "Need {}+ {} movement, but card {:?} has {}",
                                move_cost[i], move_type, card, card.movement[i]
                            ));
                        }
                    }
                }
                // Check for single-use card, unless we're using a DoubleUse token.
                is_single_use = card.single_use
                    && !mv
                        .tokens
                        .iter()
                        .any(|&i| matches!(tokens[i], BonusToken::DoubleUse));
            } else if !mv.tokens.is_empty() {
                // Token-only movement.
                let mut num_share_hex = 0;
                for &i in &mv.tokens {
                    match &tokens[i] {
                        BonusToken::Jungle(m) => {
                            if move_cost[0] > *m {
                                return Err(format!(
                                    "Need {}+ jungle movement, but token has {m}",
                                    move_cost[0]
                                ));
                            }
                        }
                        BonusToken::Desert(m) => {
                            if move_cost[1] > *m {
                                return Err(format!(
                                    "Need {}+ desert movement, but token has {m}",
                                    move_cost[1]
                                ));
                            }
                        }
                        BonusToken::Water(m) => {
                            if move_cost[2] > *m {
                                return Err(format!(
                                    "Need {}+ water movement, but token has {m}",
                                    move_cost[2]
                                ));
                            }
                        }
                        BonusToken::FreeMove => {}
                        BonusToken::ShareHex => {
                            num_share_hex += 1;
                        }
                        BonusToken::SwapSymbol => {
                            return Err(
                                "Only cards can have their symbols swapped"
                                    .into(),
                            );
                        }
                        _ => {
                            return Err(format!(
                                "Cannot use token {:?} to move",
                                tokens[mv.tokens[0]]
                            ));
                        }
                    }
                }
                if num_share_hex > 1 {
                    return Err(
                        "Can only use one ShareHex token per move".into()
                    );
                }
                let num_move_tokens = mv.tokens.len() - num_share_hex;
                if num_move_tokens != 1 {
                    return Err(format!(
                        "Must use exactly one movement token to move, got {}",
                        num_move_tokens
                    ));
                }
            } else {
                return Err("Must use cards or tokens to move".into());
            }
        }

        // Update the player's position and cards.
        let player = &mut self.players[self.curr_player_idx];
        player.position = pos;
        if is_single_use
            || (card_cost > 0
                && self.map.with_terrain(pos, Terrain::Village).is_some())
        {
            player.trash_cards(&mv.cards);
        } else {
            player.mark_played(&mv.cards);
        }
        // Clear any visited caves that are no longer adjacent.
        player
            .visited_caves
            .retain(|&cave_pos| pos.is_adjacent(cave_pos));
        // Trash any used tokens. Assumes tokens are in sorted order.
        for idx in mv.tokens.iter().rev() {
            player.tokens.swap_remove(*idx);
        }
        // Remove broken barriers from the game.
        if let Some(idx) = broken_barrier {
            let barrier = self.barriers.swap_remove(idx);
            player.broken_barriers.push(data::BrokenBarrier {
                terrain: barrier.terrain,
                cost: barrier.cost,
            });
        }
        Ok(ignore_idx)
    }

    fn handle_draw(
        &mut self,
        draw: &DrawAction,
        rng: &mut impl rand::Rng,
    ) -> Result<(), String> {
        let hand = &self.curr_player().hand;
        let hand_size = hand.len();
        let tokens = &self.curr_player().tokens;
        let num_tokens = tokens.len();
        if let Some(i) = draw.card {
            let card = hand.get(i).ok_or(format!(
                "Invalid card index {i}, given {hand_size} cards in hand"
            ))?;
            let mut is_single_use = card.single_use;
            // Check for a DoubleUse token. Any other tokens are not allowed.
            if let Some(tidx) = draw.token {
                let tok = tokens.get(tidx).ok_or(format!(
                    "Invalid token index {tidx}, given {num_tokens} tokens"
                ))?;
                if matches!(tok, BonusToken::DoubleUse) {
                    is_single_use = false;
                } else {
                    return Err(format!(
                        "Cannot use token {tok:?} when drawing cards with a card",
                    ));
                }
            }
            match card.action {
                Some(CardAction::Draw(n)) => {
                    self.players[self.curr_player_idx]
                        .fill_hand(hand_size + n, rng);
                }
                Some(CardAction::DrawAndTrash(n)) => {
                    self.players[self.curr_player_idx]
                        .fill_hand(hand_size + n, rng);
                    self.players[self.curr_player_idx].trashes += n;
                }
                _ => {
                    return Err(format!(
                        "Cannot use card {card:?} to draw more cards"
                    ));
                }
            }
            if is_single_use {
                self.players[self.curr_player_idx].trash_cards(&[i]);
            } else {
                self.players[self.curr_player_idx].mark_played(&[i]);
            }
            if let Some(tidx) = draw.token {
                self.players[self.curr_player_idx].tokens.swap_remove(tidx);
            }
        } else if let Some(i) = draw.token {
            let tok = tokens.get(i).ok_or(format!(
                "Invalid token index {i}, given {num_tokens} tokens"
            ))?;
            match tok {
                BonusToken::DrawCard => {
                    self.players[self.curr_player_idx]
                        .fill_hand(hand_size + 1, rng);
                }
                BonusToken::TrashCard => {
                    self.players[self.curr_player_idx].trashes += 1;
                }
                BonusToken::ReplaceHand => {
                    self.players[self.curr_player_idx].replace_hand(rng);
                }
                _ => {
                    return Err(format!(
                        "Cannot use token {tok:?} to draw cards"
                    ));
                }
            }
            // Remove the used token.
            self.players[self.curr_player_idx].tokens.swap_remove(i);
        } else {
            return Err(
                "Must specify a card or token to use for drawing cards".into(),
            );
        }
        Ok(())
    }

    fn handle_trash(&mut self, trash: &[usize]) -> Result<(), String> {
        let num_to_trash = trash.len();
        let num_allowed = self.curr_player().trashes;
        if num_to_trash > num_allowed {
            return Err(format!(
                "Cannot trash {} cards when {} are allowed",
                num_to_trash, num_allowed,
            ));
        }
        self.players[self.curr_player_idx].trash_cards(trash);
        self.players[self.curr_player_idx].trashes -= num_to_trash;
        Ok(())
    }

    pub fn can_visit_cave(&self, pos: AxialCoord) -> bool {
        self.bonuses
            .iter()
            .any(|(p, tokens)| *p == pos && !tokens.is_empty())
            && !self.curr_player().visited_caves.contains(&pos)
    }

    fn give_bonus(&mut self, pos: AxialCoord) -> Result<(), String> {
        let tokens = self
            .bonuses
            .iter_mut()
            .find_map(|(p, tokens)| if *p == pos { Some(tokens) } else { None })
            .ok_or(format!("No cave at {pos:?}"))?;
        let tok = tokens
            .pop()
            .ok_or(format!("No bonus tokens remaining in cave at {pos:?}"))?;
        self.players[self.curr_player_idx].tokens.push(tok);
        self.players[self.curr_player_idx].visited_caves.push(pos);
        Ok(())
    }

    /// Get the neighboring nodes of a given coordinate.
    pub fn neighbors_of(
        &self,
        coord: AxialCoord,
    ) -> impl Iterator<Item = (HexDirection, AxialCoord, &Node)> {
        self.graph.neighbors_of(&self.map, coord)
    }

    /// Get the barrier index (if any) between two boards.
    pub fn barrier_index(
        &self,
        from_board: usize,
        to_board: usize,
    ) -> Option<usize> {
        if from_board >= to_board {
            return None;
        }
        self.barriers.iter().position(|b| {
            (b.from_board == from_board && b.to_board == to_board)
                || (b.from_board == to_board && b.to_board == from_board)
        })
    }
}

fn take_card(cards: &mut Vec<BuyableCard>, idx: usize) {
    if let Some(card) = cards.get_mut(idx) {
        card.quantity -= 1;
        if card.quantity == 0 {
            cards.swap_remove(idx);
        }
    }
}

/// Finds all edges between two boards.
fn edges_between_boards(
    map: &HexMap,
    graph: &HexGraph,
    from_board: usize,
    to_board: usize,
) -> Vec<(AxialCoord, HexDirection)> {
    let mut edges = Vec::new();
    for (node_idx, (coord, node)) in map.all_nodes().enumerate() {
        if node.board_idx as usize != from_board {
            continue;
        }
        for (nbr_idx, dir) in graph.neighbor_indices(node_idx) {
            if let Some(nbr_node) = map.node_at_idx(nbr_idx)
                && nbr_node.board_idx as usize == to_board
            {
                edges.push((coord, dir));
            }
        }
    }
    edges
}

//////////////////////////
// Tests                //
//////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialization() {
        let game = GameState::new(4, "easy1", &mut rand::rng()).unwrap();
        assert_eq!(game.players.len(), 4);
        assert_eq!(game.shop.len(), 6);
        assert_eq!(game.storage.len(), 12);
    }
}
