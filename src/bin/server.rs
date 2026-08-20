//! Skeleton HTTP server letting a human play interactively against the tuned e2 AI.
//!
//! Endpoints:
//!   POST /api/games            start a new game, returns { game_id, ...GameView }
//!   GET  /api/games/:id        current state + legal actions
//!   POST /api/games/:id/actions  { "index": N } -- apply one of the last-reported actions
//!
//! Single-threaded runtime deliberately: `ExpectiMiniMaxPlayer`'s `ValueFunction` closure isn't
//! `Send` (the `Box<dyn Fn>` alias has no `+ Send` bound), so `Game` can't safely be shared across
//! OS threads today. `current_thread` sidesteps that entirely. Fine at this project's scale (one
//! AI opponent, not a high-concurrency service); revisit by adding `+ Send + Sync` to the
//! `ValueFunction`/`Player` trait bounds if this ever needs real concurrency.

use axum::{
    extract::{Path, State as AxumState},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use deckgym::web::{GameSession, GameView, SessionStore};
use deckgym::Deck;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

const DEFAULT_AI_PARAMS: &str = "tuned_params_v6.json";
const DEFAULT_AI_DEPTH: usize = 2; // e2

#[derive(Deserialize)]
struct NewGameRequest {
    #[serde(default = "default_human_deck")]
    deck_human: String,
    #[serde(default = "default_ai_deck")]
    deck_ai: String,
    #[serde(default)]
    human_seat: usize,
    seed: Option<u64>,
}

fn default_human_deck() -> String {
    "example_decks/mewtwoex.txt".to_string()
}

fn default_ai_deck() -> String {
    "example_decks/mewtwoex.txt".to_string()
}

#[derive(serde::Serialize)]
struct NewGameResponse {
    game_id: Uuid,
    #[serde(flatten)]
    view: GameView,
}

#[derive(Deserialize)]
struct SubmitActionRequest {
    index: usize,
}

async fn create_game(
    AxumState(store): AxumState<Arc<SessionStore>>,
    Json(req): Json<NewGameRequest>,
) -> Result<Json<NewGameResponse>, (StatusCode, String)> {
    let deck_human = Deck::from_file(&req.deck_human)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("deck_human: {e}")))?;
    let deck_ai = Deck::from_file(&req.deck_ai)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("deck_ai: {e}")))?;
    let seed = req.seed.unwrap_or_else(rand::random::<u64>);

    let session = GameSession::new(
        deck_human,
        deck_ai,
        req.human_seat,
        DEFAULT_AI_DEPTH,
        DEFAULT_AI_PARAMS,
        seed,
    )
    .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let view = session.view();
    let game_id = store.insert(session);
    Ok(Json(NewGameResponse { game_id, view }))
}

async fn get_game(
    AxumState(store): AxumState<Arc<SessionStore>>,
    Path(id): Path<Uuid>,
) -> Result<Json<GameView>, StatusCode> {
    store
        .with_session(id, |session| Json(session.view()))
        .ok_or(StatusCode::NOT_FOUND)
}

async fn submit_action(
    AxumState(store): AxumState<Arc<SessionStore>>,
    Path(id): Path<Uuid>,
    Json(req): Json<SubmitActionRequest>,
) -> Result<Json<GameView>, (StatusCode, String)> {
    let result = store.with_session(id, |session| {
        session.submit_action(req.index).map(|_| session.view())
    });
    match result {
        None => Err((StatusCode::NOT_FOUND, "no such game".to_string())),
        Some(Err(msg)) => Err((StatusCode::BAD_REQUEST, msg)),
        Some(Ok(view)) => Ok(Json(view)),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::init();

    let store = Arc::new(SessionStore::default());

    let app = Router::new()
        .route("/api/games", post(create_game))
        .route("/api/games/:id", get(get_game))
        .route("/api/games/:id/actions", post(submit_action))
        .layer(tower_http::cors::CorsLayer::permissive())
        .with_state(store);

    let addr = "0.0.0.0:3000";
    println!("Listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
