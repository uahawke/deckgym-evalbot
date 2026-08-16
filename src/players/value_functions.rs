// Collection of value functions for ExpectiMiniMaxPlayer
//
// Each value function evaluates a game state from a player's perspective
// and returns a score (higher is better for that player)

use log::trace;
use serde::{Deserialize, Serialize};

use crate::card_logic::get_highest_evolutions;
use crate::hooks::energy_missing;
use crate::models::{Card, EnergyType, PlayedCard};
use crate::state::GameOutcome;
use crate::State;

/// Coefficients for the parametric value function
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ValueFunctionParams {
    pub points: f64,
    pub pokemon_value: f64,
    pub hand_size: f64,
    pub deck_size: f64,
    pub active_retreat_cost: f64,
    pub active_pokemon_online_score: f64,
    pub active_safety: f64,
    pub active_has_tool: f64,
    pub is_winner: f64,
    pub turns_until_opponent_wins: f64,
    pub online_pokemon_count: f64,
    pub energy_distance_to_online: f64,
    pub opponent_discard_size: f64,
    /// Weakness matchup: +1 when the opponent's active is Weak to my active's type, -1 when
    /// mine is Weak to theirs. Weakness is +20 damage in Pocket and frequently decides whether
    /// an attack knocks out; nothing in the original 13 features represented it at all.
    #[serde(default)]
    pub active_weakness_matchup: f64,
    /// 1.0 when my active can knock out the opponent's active right now with an attack I can
    /// currently pay for.
    #[serde(default)]
    pub can_ko_opponent_active: f64,
    /// 1.0 when the opponent's active can knock out my active right now.
    #[serde(default)]
    pub opponent_can_ko_my_active: f64,
}

impl ValueFunctionParams {
    /// Baseline parameters (same as original baseline function)
    pub const fn baseline() -> Self {
        Self {
            points: 10_000.0,
            pokemon_value: 1.0,
            hand_size: 1.0,
            deck_size: 1.0,
            active_retreat_cost: 1.0,
            active_pokemon_online_score: 500.0,
            active_safety: 1.0,
            active_has_tool: 10.0,
            is_winner: 100_000.0,
            turns_until_opponent_wins: 100.0,
            online_pokemon_count: 0.0,
            energy_distance_to_online: 0.0,
            opponent_discard_size: 0.1,
            active_weakness_matchup: 0.0,
            can_ko_opponent_active: 0.0,
            opponent_can_ko_my_active: 0.0,
        }
    }

    /// Variant parameters
    pub const fn variant() -> Self {
        Self {
            points: 10_000.0,
            pokemon_value: 1.0,
            hand_size: 1.0,
            deck_size: 1.0,
            active_retreat_cost: 1.0,
            active_pokemon_online_score: 500.0,
            active_safety: 1.0,
            active_has_tool: 10.0,
            is_winner: 100_000.0,
            turns_until_opponent_wins: 100.0,
            online_pokemon_count: 0.0,
            energy_distance_to_online: 0.0,
            opponent_discard_size: 0.1,
            active_weakness_matchup: 0.0,
            can_ko_opponent_active: 0.0,
            opponent_can_ko_my_active: 0.0,
        }
    }

    /// Field order used by `to_vec` / `from_vec`. Kept explicit (rather than derived) because
    /// external optimizers index into this vector positionally; reordering it silently
    /// invalidates every params file already on disk.
    pub const FIELD_NAMES: [&'static str; 16] = [
        "points",
        "pokemon_value",
        "hand_size",
        "deck_size",
        "active_retreat_cost",
        "active_pokemon_online_score",
        "active_safety",
        "active_has_tool",
        "is_winner",
        "turns_until_opponent_wins",
        "online_pokemon_count",
        "energy_distance_to_online",
        "opponent_discard_size",
        "active_weakness_matchup",
        "can_ko_opponent_active",
        "opponent_can_ko_my_active",
    ];

    /// Load coefficients from a JSON file. This is the seam that lets an external optimizer
    /// (CMA-ES, grid search, anything) treat the simulator as a black-box fitness function
    /// without any FFI.
    pub fn from_file(path: &str) -> Result<Self, String> {
        let contents = std::fs::read_to_string(path)
            .map_err(|err| format!("Failed to read params file {path}: {err}"))?;
        serde_json::from_str(&contents)
            .map_err(|err| format!("Failed to parse params file {path}: {err}"))
    }

    pub fn to_file(&self, path: &str) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|err| format!("Failed to serialize params: {err}"))?;
        std::fs::write(path, json).map_err(|err| format!("Failed to write {path}: {err}"))
    }

    /// Flatten to a coefficient vector, in `FIELD_NAMES` order.
    pub fn to_vec(&self) -> Vec<f64> {
        vec![
            self.points,
            self.pokemon_value,
            self.hand_size,
            self.deck_size,
            self.active_retreat_cost,
            self.active_pokemon_online_score,
            self.active_safety,
            self.active_has_tool,
            self.is_winner,
            self.turns_until_opponent_wins,
            self.online_pokemon_count,
            self.energy_distance_to_online,
            self.opponent_discard_size,
            self.active_weakness_matchup,
            self.can_ko_opponent_active,
            self.opponent_can_ko_my_active,
        ]
    }

    /// Rebuild from a coefficient vector in `FIELD_NAMES` order.
    pub fn from_vec(values: &[f64]) -> Result<Self, String> {
        if values.len() != Self::FIELD_NAMES.len() {
            return Err(format!(
                "Expected {} coefficients, got {}",
                Self::FIELD_NAMES.len(),
                values.len()
            ));
        }
        Ok(Self {
            points: values[0],
            pokemon_value: values[1],
            hand_size: values[2],
            deck_size: values[3],
            active_retreat_cost: values[4],
            active_pokemon_online_score: values[5],
            active_safety: values[6],
            active_has_tool: values[7],
            is_winner: values[8],
            turns_until_opponent_wins: values[9],
            online_pokemon_count: values[10],
            energy_distance_to_online: values[11],
            opponent_discard_size: values[12],
            active_weakness_matchup: values[13],
            can_ko_opponent_active: values[14],
            opponent_can_ko_my_active: values[15],
        })
    }
}

impl Default for ValueFunctionParams {
    fn default() -> Self {
        Self::baseline()
    }
}

/// Build a `ValueFunction` closure that evaluates states with the given coefficients.
pub fn params_value_function(params: ValueFunctionParams) -> crate::players::ValueFunction {
    Box::new(move |state: &State, myself: usize| {
        parametric_value_function(state, myself, &params)
    })
}

pub fn baseline_value_function(state: &State, myself: usize) -> f64 {
    parametric_value_function(state, myself, &ValueFunctionParams::baseline())
}

/// A variant of the baseline value function
pub fn variant_value_function(state: &State, myself: usize) -> f64 {
    parametric_value_function(state, myself, &ValueFunctionParams::variant())
}

/// Parametric value function that uses the provided coefficients
pub fn parametric_value_function(
    state: &State,
    myself: usize,
    params: &ValueFunctionParams,
) -> f64 {
    let opponent = (myself + 1) % 2;
    let (my, opp) = (
        extract_features(state, myself, 1.0),
        extract_features(state, opponent, 1.0),
    );
    let score = (my.points - opp.points) * params.points
        + (my.pokemon_value - opp.pokemon_value) * params.pokemon_value
        + (my.hand_size - opp.hand_size) * params.hand_size
        + (opp.deck_size - my.deck_size) * params.deck_size
        + (-my.active_retreat_cost) * params.active_retreat_cost
        + (my.active_pokemon_online_score - opp.active_pokemon_online_score)
            * params.active_pokemon_online_score
        + (my.active_safety - opp.active_safety) * params.active_safety
        + (my.active_has_tool - opp.active_has_tool) * params.active_has_tool
        + (my.is_winner - opp.is_winner) * params.is_winner
        + (my.turns_until_opponent_wins - opp.turns_until_opponent_wins)
            * params.turns_until_opponent_wins
        + (my.online_pokemon_count - opp.online_pokemon_count) * params.online_pokemon_count
        + (my.energy_distance_to_online - opp.energy_distance_to_online)
            * params.energy_distance_to_online
        + opp.discard_size * params.opponent_discard_size
        // These three are computed from `myself`'s perspective and already carry their own sign,
        // so unlike the features above they are used directly rather than differenced.
        + active_weakness_matchup(state, myself) * params.active_weakness_matchup
        + can_ko_active(state, myself) * params.can_ko_opponent_active
        + can_ko_active(state, opponent) * params.opponent_can_ko_my_active;
    trace!("parametric_value_function: {score} (params: {params:?}, my: {my:?}, opp: {opp:?})");
    score
}

/// Features extracted from a player's game state
#[derive(Debug)]
struct Features {
    points: f64,
    pokemon_value: f64,
    hand_size: f64,
    deck_size: f64,
    active_retreat_cost: f64,
    online_pokemon_count: f64,
    energy_distance_to_online: f64,
    active_pokemon_online_score: f64,
    active_safety: f64,
    active_has_tool: f64,
    is_winner: f64,
    turns_until_opponent_wins: f64,
    discard_size: f64,
}

/// Extract features for a single player
fn extract_features(state: &State, player: usize, active_factor: f64) -> Features {
    let points = state.points[player] as f64;
    let pokemon_value = calculate_pokemon_value(state, player, active_factor);
    let hand_size = state.hands[player].len() as f64;
    let deck_size = state.decks[player].cards.len() as f64;
    let active_retreat_cost = get_active_retreat_cost(state, player) as f64;
    let (online_pokemon_count, energy_distance_to_online) =
        calculate_online_metrics(state, player, active_factor);
    let active_pokemon_online_score = calculate_active_pokemon_online_score(state, player);
    let active_safety = calculate_active_safety(state, player);
    let active_has_tool = get_active_has_tool(state, player);
    let is_winner = check_is_winner(state, player);
    let turns_until_opponent_wins = calculate_turns_until_opponent_wins(state, player);
    let discard_size = state.discard_piles[player].len() as f64;

    Features {
        points,
        pokemon_value,
        hand_size,
        deck_size,
        active_retreat_cost,
        online_pokemon_count,
        energy_distance_to_online,
        active_pokemon_online_score,
        active_safety,
        active_has_tool,
        is_winner,
        turns_until_opponent_wins,
        discard_size,
    }
}

fn get_active_retreat_cost(state: &State, player: usize) -> usize {
    state
        .maybe_get_active(player)
        .map(|card| card.card.get_retreat_cost().map(|rc| rc.len()).unwrap_or(5))
        .unwrap_or(0)
}

/// Check if active pokemon has a tool attached
fn get_active_has_tool(state: &State, player: usize) -> f64 {
    state
        .maybe_get_active(player)
        .map(|card| if card.has_tool_attached() { 1.0 } else { 0.0 })
        .unwrap_or(0.0)
}

/// Check if the player has won the game
fn check_is_winner(state: &State, player: usize) -> f64 {
    match state.winner {
        Some(GameOutcome::Win(winner)) if winner == player => 1.0,
        _ => 0.0,
    }
}

/// Calculate expected turns until opponent wins
/// Uses opponent's active damage and simulates KOs until opponent reaches 3 points
fn calculate_turns_until_opponent_wins(state: &State, player: usize) -> f64 {
    let opponent = (player + 1) % 2;

    // Find the closest pokemon to being able to deal damage (by energy requirements)
    let best_threat = state
        .enumerate_in_play_pokemon(opponent)
        .filter_map(|(_, pokemon)| {
            let best_attack = pokemon
                .card
                .get_attacks()
                .iter()
                .filter(|atk| atk.fixed_damage > 0) // Only consider attacks that deal damage
                .map(|atk| {
                    let missing = energy_missing(pokemon, &atk.energy_required, state, opponent);
                    (atk.fixed_damage, missing.len())
                })
                .min_by_key(|(damage, missing)| (*missing, u32::MAX - damage)); // Prioritize by missing energy, then by damage

            best_attack
        })
        .min_by_key(|(damage, missing)| (*missing, u32::MAX - damage)); // Find pokemon with least missing energy
    let (max_damage, missing_energy) = match best_threat {
        Some((damage, missing)) => (damage as f64, missing),
        None => return 30.0, // No pokemon can deal damage
    };

    let mut total_turns = 0.0;
    let mut opp_points = state.points[opponent];

    // If the best threat still can't attack (missing energy), factor that into the calculation
    total_turns += missing_energy as f64;

    // Calculate turns to KO my active pokemon
    if let Some(my_active) = state.maybe_get_active(player) {
        let turns_to_ko = (my_active.get_remaining_hp() as f64 / max_damage).ceil();
        total_turns += turns_to_ko;
        opp_points += my_active.card.get_knockout_points();
    }

    // Simulate KOing bench pokemon until opponent has 3+ points
    while opp_points < 3 {
        // Find the safest bench pokemon (highest hp / ko_points)
        let safest_bench = state
            .enumerate_bench_pokemon(player)
            .max_by_key(|(_, card)| {
                // if missing 1 point, just do by HP. if missing more than 1 point,
                // do by point yield hp / ko_points
                if opp_points == 2 {
                    card.get_remaining_hp()
                } else {
                    let ko_points = card.card.get_knockout_points().max(1) as u32;
                    card.get_remaining_hp() * 1000 / ko_points
                }
            });

        let Some((_, safest_pokemon)) = safest_bench else {
            break; // No more bench pokemon
        };

        let turns_to_ko = (safest_pokemon.get_remaining_hp() as f64 / max_damage).ceil();
        total_turns += turns_to_ko;
        opp_points += safest_pokemon.card.get_knockout_points();
    }

    total_turns
}

/// Calculate online pokemon metrics: (count of online pokemon, total energy distance to online)
/// A pokemon is "online" if it can use at least one attack
/// Applies active_factor bonus to the active pokemon (position 0)
fn calculate_online_metrics(state: &State, player: usize, active_factor: f64) -> (f64, f64) {
    let (online_count, total_distance) = state
        .enumerate_in_play_pokemon(player)
        .map(|(pos, card)| {
            let min_distance = card
                .card
                .get_attacks()
                .iter()
                .map(|atk| {
                    let missing = energy_missing(card, &atk.energy_required, state, player);
                    missing.len()
                })
                .min()
                .unwrap_or(0);

            let position_factor = if pos == 0 { active_factor } else { 1.0 };

            if min_distance == 0 {
                (position_factor, 0.0) // Online pokemon with position bonus
            } else {
                (0.0, min_distance as f64 * position_factor) // Offline pokemon with weighted distance
            }
        })
        .fold((0.0, 0.0), |(count, dist), (c, d)| (count + c, dist + d));

    (online_count, total_distance)
}

/// Calculate total pokemon value (HP * Energy) for a player
fn calculate_pokemon_value(state: &State, player: usize, active_factor: f64) -> f64 {
    state
        .enumerate_in_play_pokemon(player)
        .map(|(pos, card)| {
            let relevant_energy = get_relevant_energy(state, player, card);
            let hp_energy_product = card.get_remaining_hp() as f64 * (relevant_energy + 1.0);
            if pos == 0 {
                hp_energy_product * active_factor
            } else {
                hp_energy_product
            }
        })
        .sum()
}

/// Helper function to calculate relevant energy for a Pokemon
fn get_relevant_energy(state: &State, player: usize, card: &PlayedCard) -> f64 {
    let most_expensive_attack_cost: Vec<EnergyType> = card
        .card
        .get_attacks()
        .iter()
        .map(|atk| atk.energy_required.clone())
        .max()
        .unwrap_or_default();

    let missing = energy_missing(card, &most_expensive_attack_cost, state, player);

    let total = most_expensive_attack_cost.len() as f64;
    total - missing.len() as f64
}

/// Calculate active safety
/// Defined as remaining HP divided by knockout points
fn calculate_active_safety(state: &State, player: usize) -> f64 {
    let Some(active_pokemon) = state.maybe_get_active(player) else {
        return 0.0; // No safety if no active pokemon
    };

    let ko_points = active_pokemon.card.get_knockout_points() as f64;
    let hp = active_pokemon.get_remaining_hp() as f64;

    hp / ko_points.max(1.0)
}

/// Calculate online score for active pokemon (0.0 to 1.0)
/// Returns 1.0 if the active pokemon has enough energy to use the highest attack
/// of its highest evolution available in deck+hand
fn calculate_active_pokemon_online_score(state: &State, player: usize) -> f64 {
    let Some(active_pokemon) = state.maybe_get_active(player) else {
        return 0.0;
    };

    // Get all cards available in deck + hand
    let mut available_cards: Vec<Card> = state.decks[player].cards.to_vec();
    available_cards.extend(state.hands[player].iter().cloned());

    // Find the highest evolution available
    let highest_evolutions = get_highest_evolutions(&active_pokemon.card, &available_cards);

    // If no evolutions found, use the current card
    let target_card = if highest_evolutions.is_empty() {
        &active_pokemon.card
    } else {
        // Use the first highest evolution (they should all be same stage)
        &highest_evolutions[0]
    };

    // Get the highest attack energy cost from the target card
    let most_expensive_attack_cost: Vec<EnergyType> = target_card
        .get_attacks()
        .iter()
        .map(|atk| atk.energy_required.clone())
        .max()
        .unwrap_or_default();

    if most_expensive_attack_cost.is_empty() {
        return 1.0; // No attack requirements, fully online
    }

    // Calculate how much energy we have vs need
    let missing = energy_missing(active_pokemon, &most_expensive_attack_cost, state, player);
    let total_needed = most_expensive_attack_cost.len() as f64;
    let have = total_needed - missing.len() as f64;

    // Return ratio (0.0 to 1.0)
    (have / total_needed).clamp(0.0, 1.0)
}

/// Weakness matchup from `player`'s perspective: +1 if the opponent's active is Weak to my
/// active's type, -1 if mine is Weak to theirs, 0 otherwise (they cancel when both apply).
fn active_weakness_matchup(state: &State, player: usize) -> f64 {
    let opponent = (player + 1) % 2;
    let (Some(mine), Some(theirs)) = (
        state.maybe_get_active(player),
        state.maybe_get_active(opponent),
    ) else {
        return 0.0;
    };

    let mut score = 0.0;
    if let (Some(my_type), Some(their_weakness)) =
        (mine.card.get_type(), theirs.card.get_weakness())
    {
        if my_type == their_weakness {
            score += 1.0;
        }
    }
    if let (Some(their_type), Some(my_weakness)) =
        (theirs.card.get_type(), mine.card.get_weakness())
    {
        if their_type == my_weakness {
            score -= 1.0;
        }
    }
    score
}

/// 1.0 when `player`'s active can knock out the opponent's active this turn using an attack it
/// can currently pay for, 0.0 otherwise.
///
/// Deliberately uses raw `fixed_damage` plus the flat Weakness bonus rather than the full
/// `modify_damage` hook chain: that hook needs a concrete attack action and target reference,
/// and calling it at every search leaf would be far too slow. This is an approximation -- it
/// ignores Giovanni-style buffs, damage-reducing Tools, and conditional attack effects -- but
/// it captures the common case.
fn can_ko_active(state: &State, player: usize) -> f64 {
    let opponent = (player + 1) % 2;
    let (Some(attacker), Some(defender)) = (
        state.maybe_get_active(player),
        state.maybe_get_active(opponent),
    ) else {
        return 0.0;
    };

    let weakness_bonus = match (attacker.card.get_type(), defender.card.get_weakness()) {
        (Some(attack_type), Some(weakness)) if attack_type == weakness => 20,
        _ => 0,
    };
    let target_hp = defender.get_remaining_hp();

    let can_ko = attacker.card.get_attacks().iter().any(|attack| {
        if attack.fixed_damage == 0 {
            return false;
        }
        let affordable =
            energy_missing(attacker, &attack.energy_required, state, player).is_empty();
        affordable && attack.fixed_damage + weakness_bonus >= target_hp
    });

    if can_ko { 1.0 } else { 0.0 }
}
