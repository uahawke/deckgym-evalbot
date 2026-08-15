//! Evaluation harness: measures a candidate bot's strength against a gauntlet of baseline bots
//! across a set of decks, reporting win rates with Wilson score confidence intervals.
//!
//! This exists because win rates hover near 50% and single-matchup results are extremely noisy --
//! a 52% result over 200 games is indistinguishable from no improvement at all. Any claim that a
//! change made the bot stronger should come with an interval that excludes 50%.
//!
//! Every matchup is played twice with sides swapped: going first is a real advantage in this game,
//! so a one-sided sample measures seating as much as skill.
//!
//! Examples:
//!   cargo run --release --bin eval_bot -- --candidate e3 --opponents r,w,v --games 200
//!   cargo run --release --bin eval_bot -- --candidate e3 --params tuned.json --games 500
//!   cargo run --release --bin eval_bot -- --candidate v --decks-folder example_decks --json out.json

use clap::Parser;
use deckgym::players::value_functions::params_value_function;
use deckgym::players::{
    parse_player_code, AttachAttackPlayer, EndTurnPlayer, EvolutionRusherPlayer,
    ExpectiMiniMaxPlayer, MctsPlayer, Player, PlayerCode, RandomPlayer, ValueFunctionParams,
    ValueFunctionPlayer, WeightedRandomPlayer,
};
use deckgym::simulate::create_progress_bar;
use deckgym::state::GameOutcome;
use deckgym::{Deck, Simulation};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "eval_bot",
    about = "Measure a candidate bot's win rate against a gauntlet, with confidence intervals."
)]
struct Args {
    /// Candidate player code (aa, et, r, w, m, v, e<depth>, er).
    #[arg(long, default_value = "e3")]
    candidate: String,

    /// Optional JSON file of ValueFunctionParams for the candidate. Only meaningful for
    /// value-function-driven candidates (`v`, `e<depth>`); ignored otherwise.
    #[arg(long)]
    params: Option<String>,

    /// Comma-separated opponent player codes to evaluate against.
    #[arg(long, default_value = "r,w,v")]
    opponents: String,

    /// Games per (deck, opponent, side) cell. Total games = games * decks * opponents * 2.
    #[arg(long, default_value_t = 100)]
    games: u32,

    /// Folder of .txt decklists to evaluate across.
    #[arg(long, default_value = "example_decks")]
    decks_folder: String,

    /// Cap the number of decks used (0 = all). Depth-3 ExpectiMiniMax over all 32 example decks
    /// is a multi-hour job; start small and scale up once you know your per-game cost.
    #[arg(long, default_value_t = 0)]
    max_decks: usize,

    /// Base RNG seed. Each cell derives a distinct seed from this.
    #[arg(long, default_value_t = 0)]
    seed: u64,

    /// Optional path to write machine-readable results (for optimizer loops).
    #[arg(long)]
    json: Option<String>,

    /// Print only the overall win rate as a bare number. Intended for optimizers that shell out
    /// to this binary and read stdout as a fitness value.
    #[arg(long, default_value_t = false)]
    fitness_only: bool,
}

#[derive(Debug, Serialize)]
struct CellResult {
    deck: String,
    opponent: String,
    candidate_wins: u32,
    opponent_wins: u32,
    ties: u32,
    games: u32,
    win_rate: f64,
}

#[derive(Debug, Serialize)]
struct EvalReport {
    candidate: String,
    params_file: Option<String>,
    total_games: u32,
    candidate_wins: u32,
    opponent_wins: u32,
    ties: u32,
    /// Ties counted as half a win, which is the convention that keeps win rate symmetric.
    win_rate: f64,
    wilson_low: f64,
    wilson_high: f64,
    beats_coinflip: bool,
    per_cell: Vec<CellResult>,
}

/// Wilson score interval. Chosen over the normal approximation because it stays well-behaved
/// near 0 and 1 and at small n, where the naive interval can run past [0, 1] entirely.
fn wilson_interval(wins: f64, n: f64) -> (f64, f64) {
    if n <= 0.0 {
        return (0.0, 1.0);
    }
    let z = 1.96f64; // 95%
    let p = wins / n;
    let denom = 1.0 + z * z / n;
    let center = p + z * z / (2.0 * n);
    let margin = z * ((p * (1.0 - p) / n) + (z * z / (4.0 * n * n))).sqrt();
    (
        ((center - margin) / denom).max(0.0),
        ((center + margin) / denom).min(1.0),
    )
}

/// Builds a player from a code, optionally overriding the value function coefficients.
fn build_player(deck: Deck, code: &PlayerCode, params: Option<ValueFunctionParams>) -> Box<dyn Player> {
    match code {
        PlayerCode::AA => Box::new(AttachAttackPlayer { deck }),
        PlayerCode::ET => Box::new(EndTurnPlayer { deck }),
        PlayerCode::R => Box::new(RandomPlayer { deck }),
        PlayerCode::W => Box::new(WeightedRandomPlayer { deck }),
        PlayerCode::M => Box::new(MctsPlayer::new(deck, 100)),
        PlayerCode::V => Box::new(ValueFunctionPlayer { deck }),
        PlayerCode::ER => Box::new(EvolutionRusherPlayer { deck }),
        PlayerCode::E { max_depth } => Box::new(ExpectiMiniMaxPlayer {
            deck,
            max_depth: *max_depth,
            write_debug_trees: false,
            value_function: params_value_function(params.unwrap_or_default()),
        }),
        other => panic!("Player code {other:?} is not supported by eval_bot"),
    }
}

fn list_decks(folder: &str) -> Vec<(String, Deck)> {
    let mut decks = vec![];
    let entries = fs::read_dir(folder)
        .unwrap_or_else(|err| panic!("Could not read decks folder {folder}: {err}"));
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|e| e == "txt").unwrap_or(false))
        .collect();
    paths.sort();
    for path in paths {
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        match Deck::from_file(path.to_str().expect("deck path should be valid UTF-8")) {
            Ok(deck) => decks.push((name, deck)),
            Err(err) => eprintln!("Skipping {name}: {err}"),
        }
    }
    decks
}

fn main() {
    let args = Args::parse();

    let candidate_code = parse_player_code(&args.candidate)
        .unwrap_or_else(|err| panic!("Invalid candidate code: {err}"));
    let opponent_codes: Vec<PlayerCode> = args
        .opponents
        .split(',')
        .map(|s| {
            parse_player_code(s.trim()).unwrap_or_else(|err| panic!("Invalid opponent code: {err}"))
        })
        .collect();

    let params = args.params.as_ref().map(|path| {
        ValueFunctionParams::from_file(path).unwrap_or_else(|err| panic!("{err}"))
    });

    // ValueFunctionPlayer and the heuristic bots have hardcoded evaluations; only
    // ExpectiMiniMax consumes ValueFunctionParams. Silently ignoring --params made an entire
    // tuning run return identical fitness for every candidate, so refuse it outright.
    if params.is_some() && !matches!(candidate_code, PlayerCode::E { .. }) {
        panic!(
            "--params only affects ExpectiMiniMax candidates (e<depth>); '{}' has a hardcoded \
             value function and would ignore the coefficients. Use --candidate e1 for a \
             one-ply value-function player.",
            args.candidate
        );
    }

    let mut decks = list_decks(&args.decks_folder);
    assert!(!decks.is_empty(), "No .txt decks found in {}", args.decks_folder);
    if args.max_decks > 0 && decks.len() > args.max_decks {
        decks.truncate(args.max_decks);
    }

    let num_cells = decks.len() * opponent_codes.len() * 2;
    let total_planned = num_cells as u64 * args.games as u64;
    if !args.fitness_only {
        println!(
            "Plan: {} decks x {} opponents x 2 seats x {} games = {} games ({} cells)",
            decks.len(),
            opponent_codes.len(),
            args.games,
            total_planned,
            num_cells
        );
        println!("Candidate: {} | Opponents: {}", args.candidate, args.opponents);
    }
    let progress = if args.fitness_only {
        None
    } else {
        Some(create_progress_bar(total_planned))
    };

    let mut per_cell = vec![];
    let (mut total_wins, mut total_losses, mut total_ties) = (0u32, 0u32, 0u32);
    let mut cell_seed = args.seed;

    for (deck_name, deck) in &decks {
        for opponent_code in &opponent_codes {
            let (mut wins, mut losses, mut ties) = (0u32, 0u32, 0u32);

            // Play both seats: candidate as player 0, then as player 1. Going first matters
            // enough that a single-seat measurement is mostly measuring the coin toss.
            for candidate_seat in 0..2usize {
                let cand_code = candidate_code.clone();
                let opp_code = opponent_code.clone();
                let cand_params = params;

                let factory = move |deck_a: Deck, deck_b: Deck| -> Vec<Box<dyn Player>> {
                    if candidate_seat == 0 {
                        vec![
                            build_player(deck_a, &cand_code, cand_params),
                            build_player(deck_b, &opp_code, None),
                        ]
                    } else {
                        vec![
                            build_player(deck_a, &opp_code, None),
                            build_player(deck_b, &cand_code, cand_params),
                        ]
                    }
                };

                // Simulation seeds the ENTIRE batch with one value, so running N games under a
                // single Simulation replays the same game N times. Each game therefore gets its
                // own single-game Simulation with a distinct seed. Seeds are still derived
                // deterministically from --seed, so runs stay reproducible and two different
                // parameter vectors evaluated at the same --seed face identical shuffles.
                let factory = std::sync::Arc::new(factory);
                for game_idx in 0..args.games {
                    let game_seed = cell_seed
                        .wrapping_mul(1_000_003)
                        .wrapping_add(game_idx as u64);
                    let factory = factory.clone();
                    let mut simulation = Simulation::new_with_player_factory(
                        deck.clone(),
                        deck.clone(),
                        move |a, b| factory(a, b),
                        1,
                        Some(game_seed),
                        false,
                        None,
                    )
                    .expect("simulation should build");

                    for outcome in simulation.run() {
                        match outcome {
                            Some(GameOutcome::Win(winner)) if winner == candidate_seat => wins += 1,
                            Some(GameOutcome::Win(_)) => losses += 1,
                            Some(GameOutcome::Tie) | None => ties += 1,
                        }
                    }
                    if let Some(pb) = &progress {
                        pb.inc(1);
                    }
                }
                cell_seed = cell_seed.wrapping_add(1);
            }

            let games = wins + losses + ties;
            if let Some(pb) = &progress {
                pb.println(format!(
                    "  {:<24} vs {:<8} {:>5} games  {:>6.1}%",
                    deck_name,
                    format!("{opponent_code:?}"),
                    games,
                    (wins as f64 + 0.5 * ties as f64) / games.max(1) as f64 * 100.0
                ));
            }
            per_cell.push(CellResult {
                deck: deck_name.clone(),
                opponent: format!("{opponent_code:?}"),
                candidate_wins: wins,
                opponent_wins: losses,
                ties,
                games,
                win_rate: (wins as f64 + 0.5 * ties as f64) / games.max(1) as f64,
            });
            total_wins += wins;
            total_losses += losses;
            total_ties += ties;
        }
    }

    if let Some(pb) = &progress {
        pb.finish_and_clear();
    }

    let total_games = total_wins + total_losses + total_ties;
    let effective_wins = total_wins as f64 + 0.5 * total_ties as f64;
    let win_rate = effective_wins / total_games.max(1) as f64;
    let (low, high) = wilson_interval(effective_wins, total_games as f64);

    let report = EvalReport {
        candidate: args.candidate.clone(),
        params_file: args.params.clone(),
        total_games,
        candidate_wins: total_wins,
        opponent_wins: total_losses,
        ties: total_ties,
        win_rate,
        wilson_low: low,
        wilson_high: high,
        beats_coinflip: low > 0.5,
        per_cell,
    };

    if args.fitness_only {
        println!("{win_rate}");
    } else {
        println!("\nCandidate: {} (params: {:?})", report.candidate, report.params_file);
        println!("{:<24} {:<10} {:>8} {:>8}", "Deck", "Opponent", "Games", "WinRate");
        println!("{}", "-".repeat(54));
        for cell in &report.per_cell {
            println!(
                "{:<24} {:<10} {:>8} {:>7.1}%",
                cell.deck,
                cell.opponent,
                cell.games,
                cell.win_rate * 100.0
            );
        }
        println!("{}", "=".repeat(54));
        println!(
            "Overall: {:.2}% [{:.2}%, {:.2}%] over {} games ({} W / {} L / {} T)",
            win_rate * 100.0,
            low * 100.0,
            high * 100.0,
            total_games,
            total_wins,
            total_losses,
            total_ties
        );
        if report.beats_coinflip {
            println!("Interval excludes 50% -- candidate is stronger than the gauntlet.");
        } else {
            println!("Interval includes 50% -- NOT a statistically distinguishable improvement.");
        }
    }

    if let Some(path) = &args.json {
        let json = serde_json::to_string_pretty(&report).expect("report should serialize");
        fs::write(path, json).unwrap_or_else(|err| panic!("Failed to write {path}: {err}"));
    }
}
