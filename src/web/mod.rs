//! Session/controller logic for a human playing interactively against the AI over HTTP.
//!
//! The turn-by-turn control flow here is adapted from `tui::app::AppMode::Interactive`, which
//! already solved the core problem: `Game::play_tick()` assumes every seat's `Player::decision_fn`
//! can be called synchronously to get a move, which is fine for bots but not for a human waiting
//! on a web request. So a human's turn is never driven through `play_tick()` at all -- instead we
//! call `State::generate_possible_actions()` directly, hand the list to the frontend, and apply
//! whichever one the human picks via `Game::apply_action()`. The AI's turns still go through
//! `play_tick()` as normal.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;
use uuid::Uuid;

use crate::actions::Action;
use crate::models::{Card, EnergyType, PlayedCard};
use crate::players::{ExpectiMiniMaxPlayer, HumanPlayer, ValueFunctionParams};
use crate::state::GameOutcome;
use crate::{Deck, Game, State};

/// Where a human player's deck choices are read from. Distinct from `decks/train`, which holds
/// a tuning-only subset (see `decks/README.md`) -- players should see the whole curated set.
const DECKS_DIR: &str = "example_decks";

/// One deck a human player can pick, as offered by `GET /api/decks`.
#[derive(Serialize)]
pub struct DeckInfo {
    /// Server-local path, as accepted by `NewGameRequest.deck_human`/`deck_ai`.
    pub path: String,
    /// Display name derived from the filename -- deck files carry no name of their own (just an
    /// energy line and card ids), so this is a best-effort prettification, not curated data.
    pub label: String,
}

/// Lists the decks available for a human to choose from, sorted by display label.
pub fn list_decks() -> Result<Vec<DeckInfo>, String> {
    let mut decks: Vec<DeckInfo> = fs::read_dir(DECKS_DIR)
        .map_err(|e| format!("failed to read {DECKS_DIR}: {e}"))?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let file_name = entry.file_name().to_string_lossy().into_owned();
            let stem = file_name.strip_suffix(".txt")?.to_string();
            Some(DeckInfo {
                path: format!("{DECKS_DIR}/{file_name}"),
                label: prettify_deck_name(&stem),
            })
        })
        .collect();
    decks.sort_by(|a, b| a.label.cmp(&b.label));
    Ok(decks)
}

/// Turns a deck filename stem (e.g. `mewtwoex`, `giratina-darkrai`) into a display label (`Mewtwo
/// ex`, `Giratina Darkrai`). Filenames separate words with `-`/`_` except for a trailing "ex"
/// (as in the in-game "Mewtwo ex" card suffix), which this special-cases since it's otherwise
/// glued directly onto the preceding word.
fn prettify_deck_name(stem: &str) -> String {
    let spaced = stem.replace(['-', '_'], " ");
    let spaced = match spaced.strip_suffix("ex") {
        Some(head) if !head.is_empty() && !head.ends_with(' ') => format!("{head} ex"),
        _ => spaced,
    };

    spaced
        .split_whitespace()
        .map(|word| {
            if word.eq_ignore_ascii_case("ex") {
                "ex".to_string()
            } else {
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Builds the two `Player` trait objects a `GameSession` drives -- shared between starting a
/// fresh game (`GameSession::new`) and rebuilding one from a persisted snapshot
/// (`GameSession::from_persisted`), since a `Box<dyn Player>` (the AI's holds a closure) can't be
/// serialized and has to be reconstructed from its deck + params path either way.
fn build_players(
    deck_human: Deck,
    deck_ai: Deck,
    human_seat: usize,
    ai_depth: usize,
    ai_params_path: &str,
) -> Result<Vec<Box<dyn crate::players::Player>>, String> {
    let ai_params = ValueFunctionParams::from_file(ai_params_path)?;
    let ai_player: Box<ExpectiMiniMaxPlayer> = Box::new(ExpectiMiniMaxPlayer {
        deck: deck_ai,
        max_depth: ai_depth,
        write_debug_trees: false,
        value_function: crate::players::params_value_function(ai_params),
    });
    let human_player: Box<HumanPlayer> = Box::new(HumanPlayer { deck: deck_human });

    // decision_fn is only ever called on whichever player occupies the AI seat -- the human
    // seat's HumanPlayer is present purely to satisfy Game::new's Vec<Box<dyn Player>>
    // (it needs get_deck() for initial setup); its decision_fn (which blocks on stdin) is
    // never invoked, since we never call play_tick() while it's the human's turn.
    Ok(if human_seat == 0 {
        vec![human_player as Box<dyn crate::players::Player>, ai_player]
    } else {
        vec![ai_player as Box<dyn crate::players::Player>, human_player]
    })
}

/// A single in-progress game between a human and the AI.
pub struct GameSession {
    game: Game<'static>,
    /// Which seat (0 or 1) the human occupies. The other seat always auto-plays via `play_tick`.
    human_seat: usize,
    /// The AI's search depth (2 for e2, 3 for e3/"hard mode"), surfaced back to the frontend so
    /// it can show which difficulty is actually running.
    ai_depth: usize,
    current_actor: usize,
    possible_actions: Vec<Action>,
    // The rest of these fields exist only so `to_persisted` can rebuild an equivalent session
    // (players, value function) after a server restart -- see `PersistedSession`.
    deck_human_path: String,
    deck_ai_path: String,
    ai_params_path: String,
    seed: u64,
}

impl GameSession {
    /// Starts a new game. `ai_params_path` is the champion coefficients file for the AI's value
    /// function (e.g. `tuned_params_v6.json`); `ai_depth` is the search depth (2 for e2).
    pub fn new(
        deck_human_path: &str,
        deck_ai_path: &str,
        human_seat: usize,
        ai_depth: usize,
        ai_params_path: &str,
        seed: u64,
    ) -> Result<Self, String> {
        if human_seat > 1 {
            return Err(format!("human_seat must be 0 or 1, got {human_seat}"));
        }
        let deck_human = Deck::from_file(deck_human_path).map_err(|e| format!("deck_human: {e}"))?;
        let deck_ai = Deck::from_file(deck_ai_path).map_err(|e| format!("deck_ai: {e}"))?;
        let players = build_players(deck_human, deck_ai, human_seat, ai_depth, ai_params_path)?;

        let game = Game::new(players, seed);
        let mut session = GameSession {
            game,
            human_seat,
            ai_depth,
            current_actor: 0,
            possible_actions: vec![],
            deck_human_path: deck_human_path.to_string(),
            deck_ai_path: deck_ai_path.to_string(),
            ai_params_path: ai_params_path.to_string(),
            seed,
        };
        session.advance();
        Ok(session)
    }

    /// Rebuilds a session from a previous run's `to_persisted()` snapshot, picking up exactly
    /// where the game state left off. Reloads decks/value-function params from the same paths
    /// used originally, since neither the players (trait objects, one holding a closure) nor the
    /// `Game`'s RNG can be serialized directly.
    ///
    /// Known limitation: the restored `Game` seeds a fresh RNG from the original `seed` rather
    /// than resuming its exact prior state (which isn't recoverable -- see above), so any coin
    /// flips/shuffles after a restart restart that seed's sequence from the beginning rather than
    /// continuing it. Not a correctness issue (still valid randomness), just an imperfect reuse of
    /// the sequence prefix across a restart.
    fn from_persisted(p: PersistedSession) -> Result<Self, String> {
        if p.human_seat > 1 {
            return Err(format!("human_seat must be 0 or 1, got {}", p.human_seat));
        }
        let deck_human =
            Deck::from_file(&p.deck_human_path).map_err(|e| format!("deck_human: {e}"))?;
        let deck_ai = Deck::from_file(&p.deck_ai_path).map_err(|e| format!("deck_ai: {e}"))?;
        let players = build_players(
            deck_human,
            deck_ai,
            p.human_seat,
            p.ai_depth,
            &p.ai_params_path,
        )?;

        let game = Game::from_state(p.state, players, p.seed);
        let mut session = GameSession {
            game,
            human_seat: p.human_seat,
            ai_depth: p.ai_depth,
            current_actor: 0,
            possible_actions: vec![],
            deck_human_path: p.deck_human_path,
            deck_ai_path: p.deck_ai_path,
            ai_params_path: p.ai_params_path,
            seed: p.seed,
        };
        session.advance();
        Ok(session)
    }

    /// Snapshot of everything needed to resume this session later via `from_persisted`.
    fn to_persisted(&self) -> PersistedSession {
        PersistedSession {
            deck_human_path: self.deck_human_path.clone(),
            deck_ai_path: self.deck_ai_path.clone(),
            human_seat: self.human_seat,
            ai_depth: self.ai_depth,
            ai_params_path: self.ai_params_path.clone(),
            seed: self.seed,
            state: self.game.get_state_clone(),
        }
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
            ai_depth: self.ai_depth,
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
                    label: action.action.describe(),
                })
                .collect(),
            state: PlayerStateView::from_state(&state, self.human_seat),
        }
    }
}

/// Everything needed to rebuild a `GameSession` after a restart. Doesn't include
/// `current_actor`/`possible_actions` -- those are a pure function of `state` (via
/// `State::generate_possible_actions`), so `from_persisted`'s call to `advance()` regenerates them
/// instead of trusting a second, potentially stale copy.
#[derive(Serialize, Deserialize)]
struct PersistedSession {
    deck_human_path: String,
    deck_ai_path: String,
    human_seat: usize,
    ai_depth: usize,
    ai_params_path: String,
    seed: u64,
    state: State,
}

/// What the frontend gets back after starting a game or submitting an action.
#[derive(Serialize)]
pub struct GameView {
    pub turn_count: u8,
    pub current_actor: usize,
    pub human_seat: usize,
    pub ai_depth: usize,
    pub is_human_turn: bool,
    pub is_game_over: bool,
    pub winner: Option<GameOutcome>,
    pub points: [u8; 2],
    pub possible_actions: Vec<ActionView>,
    pub state: PlayerStateView,
}

#[derive(Serialize)]
pub struct ActionView {
    pub index: usize,
    pub actor: usize,
    pub label: String,
}

/// `State` filtered to what `viewer` is actually allowed to see. The raw `State` holds both
/// players' full hands and deck contents (it has to, internally, to simulate the game) -- serializing
/// it as-is over HTTP would hand the human's browser the AI's hidden hand and deck order, readable
/// from the network tab regardless of anything the AI's search does. Everything else in TCG Pocket
/// (bench/active Pokemon, discard piles, points, stadium) is public to both players and passed
/// through unfiltered.
#[derive(Serialize)]
pub struct PlayerStateView {
    pub winner: Option<GameOutcome>,
    pub points: [u8; 2],
    pub turn_count: u8,
    pub current_player: usize,
    /// The viewer's own energy zone, in full.
    pub my_energy_zone: EnergyZoneView,
    /// The opponent's energy zone. `next` is deliberately omitted here -- unconfirmed whether
    /// TCG Pocket shows an opponent's upcoming energy type or only your own; defaulting to
    /// hidden (the safe direction) until that's checked against the real game's rules.
    pub opponent_energy_current: Option<EnergyType>,
    /// The viewer's own hand, in full.
    pub my_hand: Vec<Card>,
    pub opponent_hand_size: usize,
    /// [my deck size, opponent deck size] -- card counts only; nobody sees deck contents or
    /// order, including the deck's own owner.
    pub deck_sizes: [usize; 2],
    pub discard_piles: [Vec<Card>; 2],
    pub discard_energies: [Vec<EnergyType>; 2],
    /// [my in-play Pokemon, opponent's in-play Pokemon] -- index 0 of each is the active, 1..4
    /// the bench. Always public in TCG Pocket.
    pub in_play_pokemon: [[Option<PlayedCard>; 4]; 2],
    pub active_stadium: Option<Card>,
    pub active_stadium_owner: Option<usize>,
}

#[derive(Serialize)]
pub struct EnergyZoneView {
    pub current: Option<EnergyType>,
    pub next: Option<EnergyType>,
}

impl PlayerStateView {
    fn from_state(state: &State, viewer: usize) -> Self {
        let opponent = (viewer + 1) % 2;
        PlayerStateView {
            winner: state.winner,
            points: state.points,
            turn_count: state.turn_count,
            current_player: state.current_player,
            my_energy_zone: EnergyZoneView {
                current: state.energy_zone[viewer].current,
                next: state.energy_zone[viewer].next,
            },
            opponent_energy_current: state.energy_zone[opponent].current,
            my_hand: state.hands[viewer].clone(),
            opponent_hand_size: state.hands[opponent].len(),
            deck_sizes: [
                state.decks[viewer].cards.len(),
                state.decks[opponent].cards.len(),
            ],
            discard_piles: [
                state.discard_piles[viewer].clone(),
                state.discard_piles[opponent].clone(),
            ],
            discard_energies: [
                state.discard_energies[viewer].clone(),
                state.discard_energies[opponent].clone(),
            ],
            in_play_pokemon: [
                state.in_play_pokemon[viewer].clone(),
                state.in_play_pokemon[opponent].clone(),
            ],
            active_stadium: state.active_stadium.clone(),
            active_stadium_owner: state.active_stadium_owner,
        }
    }
}

/// Where session snapshots are written, one JSON file per game keyed by its id. Single-process
/// still (a second server instance would need to share this directory *and* serialize access to
/// it -- not done here), but games now survive a restart of this one.
const SESSIONS_DIR: &str = "sessions";

/// In-memory session store, keyed by game id, backed by on-disk snapshots in `SESSIONS_DIR` so
/// in-progress games survive a server restart.
pub struct SessionStore {
    sessions: Mutex<HashMap<Uuid, GameSession>>,
}

impl SessionStore {
    /// Restores any sessions persisted by a previous run. Best-effort: a session file that fails
    /// to parse, or whose deck/params files have since moved, is dropped with a warning rather
    /// than blocking startup -- a stale unplayable session isn't worth failing the whole server
    /// over.
    pub fn load() -> Self {
        let mut sessions = HashMap::new();
        if let Ok(entries) = fs::read_dir(SESSIONS_DIR) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let Some(id) = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .and_then(|s| Uuid::parse_str(s).ok())
                else {
                    continue;
                };
                let restored = fs::read_to_string(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|s| serde_json::from_str::<PersistedSession>(&s).map_err(|e| e.to_string()))
                    .and_then(GameSession::from_persisted);
                match restored {
                    Ok(session) => {
                        sessions.insert(id, session);
                    }
                    Err(e) => log::warn!("dropping unrestorable session {id}: {e}"),
                }
            }
        }
        SessionStore {
            sessions: Mutex::new(sessions),
        }
    }

    pub fn insert(&self, session: GameSession) -> Uuid {
        let id = Uuid::new_v4();
        persist(id, &session);
        self.sessions.lock().unwrap().insert(id, session);
        id
    }

    pub fn with_session<T>(&self, id: Uuid, f: impl FnOnce(&mut GameSession) -> T) -> Option<T> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions.get_mut(&id)?;
        let result = f(session);
        persist(id, session);
        Some(result)
    }
}

/// Writes a session's snapshot to disk. Best-effort: a failed write means that particular game
/// won't survive a restart, which isn't worth failing the in-flight HTTP request over.
fn persist(id: Uuid, session: &GameSession) {
    if let Err(e) = fs::create_dir_all(SESSIONS_DIR) {
        log::warn!("failed to create {SESSIONS_DIR}: {e}");
        return;
    }
    match serde_json::to_string(&session.to_persisted()) {
        Ok(json) => {
            if let Err(e) = fs::write(format!("{SESSIONS_DIR}/{id}.json"), json) {
                log::warn!("failed to persist session {id}: {e}");
            }
        }
        Err(e) => log::warn!("failed to serialize session {id}: {e}"),
    }
}
