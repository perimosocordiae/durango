use blau_api::{DynSafeGameAPI, GameAPI, PlayerInfo, Result};
use serde::{Deserialize, Serialize};

use crate::{
    agent::{Agent, create_agent},
    cards::{BuyableCard, Card},
    data::{AxialCoord, Barrier, BonusToken, HexMap},
    game::{ActionOutcome, GameState, PlayerAction},
    player::Player,
};

/// Parameters for game initialization.
#[derive(Deserialize)]
struct GameParams {
    // Named layout to use, e.g. "easy1"
    named_layout: String,
}

/// A view of another player's public information.
#[derive(Serialize)]
pub struct OtherPlayerView<'a> {
    player_idx: usize,
    position: AxialCoord,
    hand: &'a [Card],
    tokens: &'a [BonusToken],
}

/// A view of the game state for a specific player.
#[derive(Serialize)]
pub struct PlayerView<'a> {
    map: &'a HexMap,
    barriers: &'a [Barrier],
    player: &'a Player,
    other_players: Vec<OtherPlayerView<'a>>,
    bonuses: Vec<(&'a AxialCoord, usize)>,
    shop: &'a [BuyableCard],
    storage: &'a [BuyableCard],
    round_idx: usize,
    curr_player_idx: usize,
    winner: Option<usize>,
}

/// Final data to store for viewing completed games.
#[derive(Serialize, Deserialize)]
struct FinalState {
    game: GameState,
    scores: Vec<i32>,
    history: Vec<Vec<Vec<AxialCoord>>>,
}

pub struct DurangoAPI {
    // Current game state
    state: GameState,
    // Player IDs in the same order as agents
    player_ids: Vec<String>,
    // None if human player
    agents: Vec<Option<Box<dyn Agent + Send>>>,
    // Player i, turn j, position k
    history: Vec<Vec<Vec<AxialCoord>>>,
    // Indicates if the game is over
    game_over: bool,
}

impl DurangoAPI {
    fn view(&self, player_idx: usize) -> Result<String> {
        let game = &self.state;
        let winner = if game.is_game_over() {
            game.player_scores()
                .iter()
                .enumerate()
                .max_by_key(|&(_, score)| score)
                .map(|(i, _)| i)
        } else {
            None
        };
        let other_players = game
            .players
            .iter()
            .enumerate()
            .filter_map(|(idx, p)| {
                if idx != player_idx {
                    Some(OtherPlayerView {
                        player_idx: idx,
                        position: p.position,
                        hand: &p.hand,
                        tokens: &p.tokens,
                    })
                } else {
                    None
                }
            })
            .collect();
        let view = PlayerView {
            map: &game.map,
            barriers: &game.barriers,
            player: &game.players[player_idx],
            other_players,
            bonuses: game.bonus_counts(),
            shop: &game.shop,
            storage: &game.storage,
            round_idx: game.round_idx,
            curr_player_idx: game.curr_player_idx,
            winner,
        };
        Ok(serde_json::to_string(&view)?)
    }
    fn do_action<F: FnMut(&str, &str)>(
        &mut self,
        action: &PlayerAction,
        mut notice_cb: F,
    ) -> Result<()> {
        // Take the action.
        let mut ignored_idx = None;
        match self.state.process_action(action)? {
            ActionOutcome::Ok => {}
            ActionOutcome::GameOver => {
                self.game_over = true;
            }
            ActionOutcome::IgnoreMoveIdx(idx) => {
                ignored_idx = Some(idx);
            }
        }
        // If this was a move, update history.
        if let PlayerAction::Move(mv) = action {
            let my_turns = &mut self.history[self.state.curr_player_idx];
            let mut prev_pos =
                *my_turns.iter().rev().find_map(|turn| turn.last()).unwrap();
            while self.state.round_idx >= my_turns.len() {
                my_turns.push(vec![]);
            }
            let curr_turn = my_turns.get_mut(self.state.round_idx).unwrap();
            for (i, dir) in mv.path.iter().enumerate() {
                if ignored_idx != Some(i) {
                    prev_pos = dir.neighbor_coord(prev_pos);
                    curr_turn.push(prev_pos);
                }
            }
        }
        // Notify all human players of the action.
        for idx in self.human_player_idxs() {
            notice_cb(self.player_ids[idx].as_str(), self.view(idx)?.as_str());
        }
        Ok(())
    }
    fn human_player_idxs(&self) -> impl Iterator<Item = usize> + '_ {
        self.agents.iter().enumerate().filter_map(|(idx, agent)| {
            if agent.is_none() { Some(idx) } else { None }
        })
    }
    fn process_agents<F: FnMut(&str, &str)>(
        &mut self,
        mut notice_cb: F,
    ) -> Result<()> {
        while !self.game_over
            && let Some(ai) = &self.agents[self.state.curr_player_idx]
        {
            let action = ai.choose_action(&self.state);
            self.do_action(&action, &mut notice_cb)?;
        }
        Ok(())
    }
}
impl GameAPI for DurangoAPI {
    fn init(players: &[PlayerInfo], params: Option<&str>) -> Result<Self> {
        let params: GameParams = match params {
            Some(p) => serde_json::from_str(p)?,
            None => GameParams {
                named_layout: "easy1".to_string(),
            },
        };
        let mut rng = rand::rng();
        let state =
            GameState::new(players.len(), &params.named_layout, &mut rng)?;
        let player_ids = players.iter().map(|p| p.id.clone()).collect();
        let agents = players
            .iter()
            .map(|p| p.level.map(|lvl| create_agent(1 + lvl as usize)))
            .collect();
        let history = state
            .player_positions()
            .into_iter()
            .map(|pos| vec![vec![pos]])
            .collect();
        Ok(Self {
            state,
            player_ids,
            agents,
            history,
            game_over: false,
        })
    }

    fn restore(player_info: &[PlayerInfo], final_state: &str) -> Result<Self> {
        let fs: FinalState = serde_json::from_str(final_state)?;
        Ok(Self {
            state: fs.game,
            player_ids: player_info.iter().map(|p| p.id.clone()).collect(),
            agents: vec![],
            history: fs.history,
            game_over: true,
        })
    }

    fn start<F: FnMut(&str, &str)>(
        &mut self,
        game_id: i64,
        mut notice_cb: F,
    ) -> Result<()> {
        let msg = format!(r#"{{"action": "start", "game_id": {game_id}}}"#);
        for idx in self.human_player_idxs() {
            notice_cb(self.player_ids[idx].as_str(), &msg);
        }
        // Advance to wait for the next player action.
        self.process_agents(notice_cb)?;
        Ok(())
    }

    fn process_action<F: FnMut(&str, &str)>(
        &mut self,
        action: &str,
        mut notice_cb: F,
    ) -> Result<()> {
        if self.game_over {
            return Err("Game is over".into());
        }
        let action: PlayerAction = serde_json::from_str(action)?;
        self.do_action(&action, &mut notice_cb)?;
        // Advance to wait for the next player action.
        self.process_agents(&mut notice_cb)?;
        Ok(())
    }
}

impl DynSafeGameAPI for DurangoAPI {
    fn is_game_over(&self) -> bool {
        self.game_over
    }

    fn final_state(&self) -> Result<String> {
        if !self.game_over {
            return Err("Game is not finished".into());
        }
        let fs = FinalState {
            game: self.state.clone(),
            scores: self.state.player_scores(),
            history: self.history.clone(),
        };
        Ok(serde_json::to_string(&fs)?)
    }

    fn player_view(&self, player_id: &str) -> Result<String> {
        let player_idx = self
            .player_ids
            .iter()
            .position(|id| id == player_id)
            .ok_or("Unknown player ID")?;
        self.view(player_idx)
    }

    fn current_player_id(&self) -> &str {
        self.player_ids[self.state.curr_player_idx].as_str()
    }

    fn player_scores(&self) -> Vec<i32> {
        self.state.player_scores()
    }
}

#[test]
fn exercise_api() {
    let players = vec![
        PlayerInfo::human("foo".into()),
        PlayerInfo::ai("bot".into(), 1),
    ];
    let mut game: DurangoAPI =
        GameAPI::init(&players, Some(r#"{"named_layout": "first"}"#)).unwrap();
    game.start(1234, |id, msg| {
        assert_eq!(id, "foo");
        assert_eq!(msg, "{\"action\": \"start\", \"game_id\": 1234}");
    })
    .unwrap();

    let view_json = game.player_view("foo").unwrap();
    assert!(view_json.starts_with("{"));

    let mut num_notices = 0;
    game.process_action("\"FinishTurn\"", |id, msg| {
        assert_eq!(id, "foo");
        assert!(msg.starts_with("{"));
        num_notices += 1;
    })
    .unwrap();
    // There should be between 2 and 6 notices,
    // depending on how many actions the AI took.
    assert!(
        (2..=6).contains(&num_notices),
        "num_notices={num_notices} out of bounds [2, 6]",
    );
}

#[test]
fn self_play() {
    let players = vec![
        PlayerInfo::ai("bot1".into(), 1),
        PlayerInfo::ai("bot2".into(), 1),
    ];
    let mut game: DurangoAPI =
        GameAPI::init(&players, Some(r#"{"named_layout": "easy1"}"#)).unwrap();
    // Run until game over
    game.start(1234, |_, _| {}).unwrap();
    assert!(game.is_game_over());
    // Check that the history matches the final positions
    let final_positions = game.state.player_positions();
    for (idx, pos) in final_positions.iter().enumerate() {
        let history = &game.history[idx];
        let last_pos = history.last().unwrap().last().unwrap();
        assert_eq!(pos, last_pos);
    }
}
