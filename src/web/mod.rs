//! Session/controller logic for a human playing interactively against the AI over HTTP.
//!
//! The turn-by-turn control flow here is adapted from `tui::app::AppMode::Interactive`, which
//! already solved the core problem: `Game::play_tick()` assumes every seat's `Player::decision_fn`
//! can be called synchronously to get a move, which is fine for bots but not for a human waiting
//! on a web request. So a human's turn is never driven through `play_tick()` at all -- instead we
//! call `State::generate_possible_actions()` directly, hand the list to the frontend, and apply
//! whichever one the human picks via `Game::apply_action()`. The AI's turns still go through
//! `play_tick()` as normal.

use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

use crate::actions::Action;
use crate::players::{ExpectiMiniMaxPlayer, HumanPlayer, ValueFunctionParams};
use crate::{Deck, Game, State};

/// A single in-progress game between a human and the AI.
pub struct GameSession {
    game: Game<'static>,
    /// Which seat (0 or 1) the human occupies. The other seat always auto-plays via `play_tick`.
    human_seat: usize,
    current_actor: usize,
    possible_actions: Vec<Action>,
}

impl GameSession {
    /// Starts a new game. `ai_params_path` is the champion coefficients file for the AI's value
    /// function (e.g. `tuned_params_v6.json`); `ai_depth` is the search depth (2 for e2).
    pub fn new(
        deck_human: Deck,
        deck_ai: Deck,
        human_seat: usize,
        ai_depth: usize,
        ai_params_path: &str,
        seed: u64,
    ) -> Result<Self, String> {
        if human_seat > 1 {
            return Err(format!("human_seat must be 0 or 1, got {human_seat}"));
        }
        let ai_params = ValueFunctionParams::from_file(ai_params_path)?;
        let ai_player: Box<ExpectiMiniMaxPlayer> = Box::new(ExpectiMiniMaxPlayer {
            deck: deck_ai.clone(),
            max_depth: ai_depth,
            write_debug_trees: false,
            value_function: crate::players::params_value_function(ai_params),
        });
        let human_player: Box<HumanPlayer> = Box::new(HumanPlayer {
            deck: deck_human.clone(),
        });

        // decision_fn is only ever called on whichever player occupies the AI seat -- the human
        // seat's HumanPlayer is present purely to satisfy Game::new's Vec<Box<dyn Player>>
        // (it needs get_deck() for initial setup); its decision_fn (which blocks on stdin) is
        // never invoked, since we never call play_tick() while it's the human's turn.
        let players = if human_seat == 0 {
            vec![human_player as Box<dyn crate::players::Player>, ai_player]
        } else {
            vec![ai_player as Box<dyn crate::players::Player>, human_player]
        };

        let game = Game::new(players, seed);
        let mut session = GameSession {
            game,
            human_seat,
            current_actor: 0,
            possible_actions: vec![],
        };
        session.advance();
        Ok(session)
    }

    /// Auto-plays the AI's turns (and any stack-driven sub-decisions on its side) until it's the
    /// human's turn, or the game ends.
    fn advance(&mut self) {
        loop {
            if self.game.is_game_over() {
                self.possible_actions = vec![];
                break;
            }
            let (actor, actions) = self.game.get_state_clone().generate_possible_actions();
            self.current_actor = actor;
            self.possible_actions = actions;
            if actor == self.human_seat {
                break;
            }
            self.game.play_tick();
        }
    }

    /// Applies the human's chosen action (by index into the last-reported possible_actions) and
    /// advances the game. Errors if it isn't currently the human's turn or the index is invalid.
    pub fn submit_action(&mut self, action_index: usize) -> Result<(), String> {
        if self.game.is_game_over() {
            return Err("game is already over".to_string());
        }
        if self.current_actor != self.human_seat {
            return Err("not the human's turn".to_string());
        }
        let action = self
            .possible_actions
            .get(action_index)
            .ok_or_else(|| format!("invalid action index {action_index}"))?
            .clone();
        self.game.apply_action(&action);
        self.advance();
        Ok(())
    }

    pub fn view(&self) -> GameView {
        let state = self.game.get_state_clone();
        GameView {
            turn_count: state.turn_count,
            current_actor: self.current_actor,
            human_seat: self.human_seat,
            is_human_turn: !self.game.is_game_over() && self.current_actor == self.human_seat,
            is_game_over: self.game.is_game_over(),
            winner: state.winner,
            points: state.points,
            possible_actions: self
                .possible_actions
                .iter()
                .enumerate()
                .map(|(index, action)| ActionView {
                    index,
                    actor: action.actor,
                    label: action.action.to_string(),
                })
                .collect(),
            state,
        }
    }
}

/// What the frontend gets back after starting a game or submitting an action. `state` is the
/// full internal State (already Serialize) -- a real frontend will likely want a slimmer,
/// purpose-built view, but exposing the whole thing is enough to build against for now.
#[derive(Serialize)]
pub struct GameView {
    pub turn_count: u8,
    pub current_actor: usize,
    pub human_seat: usize,
    pub is_human_turn: bool,
    pub is_game_over: bool,
    pub winner: Option<crate::state::GameOutcome>,
    pub points: [u8; 2],
    pub possible_actions: Vec<ActionView>,
    pub state: State,
}

#[derive(Serialize)]
pub struct ActionView {
    pub index: usize,
    pub actor: usize,
    pub label: String,
}

/// In-memory session store, keyed by game id. Single-process, lost on restart -- fine for a
/// skeleton; swap for a real store (Redis, a DB) before this needs to survive restarts or run
/// behind more than one server process.
#[derive(Default)]
pub struct SessionStore {
    sessions: Mutex<HashMap<Uuid, GameSession>>,
}

impl SessionStore {
    pub fn insert(&self, session: GameSession) -> Uuid {
        let id = Uuid::new_v4();
        self.sessions.lock().unwrap().insert(id, session);
        id
    }

    pub fn with_session<T>(&self, id: Uuid, f: impl FnOnce(&mut GameSession) -> T) -> Option<T> {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.get_mut(&id).map(f)
    }
}
