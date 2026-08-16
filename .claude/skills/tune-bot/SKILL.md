---
name: tune-bot
description: Measure and improve the strength of AI players in this Pokemon TCG Pocket engine, using the eval_bot gauntlet harness and the CMA-ES value function tuner. Use when comparing bots, tuning ValueFunctionParams, or making any claim that a change made the engine's AI stronger.
---

# Tuning and Evaluating Bots

This skill covers measuring bot strength and improving the parametric value function. The
measurement half matters more than the tuning half: win rates in this engine sit near 50% between
comparable bots and near 90% against weak ones, so it is very easy to produce confident,
wrong conclusions. Most of the rules below exist because a specific mistake already happened.

Read `src/players/mod.rs`, `src/players/value_functions.rs`, and `src/simulate.rs` before
starting.

## Core workflow

Always in this order. Skipping step 1 is how bad results get published.

1. **Establish a baseline** with `eval_bot` on the exact configuration you will compare against.
2. **Change one thing** (coefficients, features, search depth).
3. **Validate on held-out decks and unseen seeds**, never on the decks you tuned against.
4. **Compare Wilson intervals**, not point estimates.

## Measurement rules

These are not style preferences. Each one corresponds to a real failure mode.

- **Never batch games under one seed.** `Simulation::new_with_*(..., Some(seed), ...)` seeds the
  entire batch, so N games under one `Simulation` are N byte-identical replays of the same game.
  A run reporting "800 games" this way has an effective sample size of 4. `eval_bot` avoids this
  by constructing a single-game `Simulation` per game with a derived seed. If you write new
  harness code, do the same.
  - Tell: win rates land on exactly 50.0% or 100.0%.
- **Always play both seats.** Going first is a real advantage; a one-sided sample measures the
  coin toss as much as skill. `eval_bot` swaps seats automatically.
- **Only `e<depth>` candidates consume `--params`.** `ValueFunctionPlayer` (`v`) and the heuristic
  bots have hardcoded evaluations and silently ignore coefficients. `eval_bot` now panics rather
  than ignoring this, but be aware when writing new players.
  - Tell: every candidate in a tuning run returns identical fitness.
- **Ties are turn-limit timeouts.** `State` declares a tie past turn 30, and ~20% of games against
  weak opponents time out. Scoring a tie as half a win tells an optimizer that stalling to turn 30
  is as good as winning half your games, which is a local optimum it will find. Use
  `wins / (wins + losses)` with an explicit tie penalty.
- **Report direction, not just significance.** An interval that fails to exclude 50% might be
  "no difference" or might be "significantly worse". Distinguish them.

## Verifying the harness itself

Before trusting any number from a new or modified harness, run a mirror match:

```bash
cargo run --release --bin eval_bot -- --candidate e1 --opponents e1 --games 200 --max-decks 4
```

Identical bots on both sides should land near 50% with the interval straddling it. If it does
not, there is a seat or seeding bug and every downstream measurement inherits it.

## Choosing an opponent (the objective matters more than the optimizer)

Against `r` and `w`, every reasonable bot wins 85-92%. In that compressed regime "improvement"
mostly means beating a weak opponent more thoroughly, which is not the same skill as beating a
competent one. Tuning against weak opponents produced params that scored *better* on training
fitness and *worse* in real play.

**Tune head-to-head against a fixed strong reference instead:**

```bash
--opponents e1 --opponent-params <current_champion>.json
```

This is centered on 50% with full dynamic range in both directions. Baseline fitness should print
near 0.45, not near 0.80. If it prints near 0.80, the objective is saturated and the run is
worthless.

Do not tune against the candidate itself: self-play makes fitness non-stationary.

## Running the tuner

```bash
python3 -m venv .venv && source .venv/bin/activate
pip install cma numpy

python -u python/tune_value_function.py \
  --generations 20 --popsize 12 --games 50 --decks 12 \
  --decks-folder <training_decks_only> \
  --opponents e1 --opponent-params <champion>.json \
  --out tuned_params_vN.json
```

- Use `--candidate e1` (the default). `e2`/`e3` are 10-100x slower and infeasible in a loop.
- Hold decks out of the training folder so validation is honest.
- Use `nohup ... &` with `python -u` for long runs; without `-u`, stdout block-buffers and the log
  stays empty for hours.
- New coefficients should default to `0.0` so the baseline is numerically unchanged and any gain
  is attributable to the feature rather than a moved starting point.

Scale normalization is required: raw coefficients span 10,000 to 0.1, and CMA-ES uses one step
size across all dimensions. The tuner optimizes `param = baseline + z * scale`.

`is_winner` is frozen. At 100,000 it is a sentinel that makes terminal wins dominate, not a
tradeoff; perturbing it corrupts the ordering.

## Validating a result

```bash
cargo run --release --bin eval_bot -- \
  --candidate e1 --params tuned_params_vN.json \
  --opponents e1 --opponent-params <champion>.json \
  --games 150 --max-decks 10 --seed 999999
```

- Watch the **train/test gap**. Training fitness well above held-out win rate means overfitting;
  the fix is more decks and more games per evaluation, not more generations.
- Watch the **per-deck spread**, not only the aggregate. A bot that gains 15 points on one
  archetype and loses 15 on another can show a flat overall number while being much worse. Tighter
  spread at equal mean is the better bot.
- If a coefficient comes out with a surprising sign, test it: negate that coefficient and run it
  head-to-head against the original. If the flipped version wins, there is a sign bug in the
  feature; if it loses, the sign is real and worth understanding.

## Known findings

- **Value function params are depth-specific.** Coefficients tuned at `e1` gave 63% against the
  hand-tuned baseline at `e1`, but no significant edge at `e2`. Deeper search evaluates a
  different distribution of states (end-of-turn positions rather than mid-turn snapshots), so
  coefficients do not transfer. Retune per deployment depth.
- **`hand_size` should be negative.** Every tuning run flips it from the baseline's `+1.0`. Cards
  in hand are unplayed resources.
- **`energy_distance_to_online` matters** and ships disabled at `0.0`. Every run switches it on.
- **`online_pokemon_count` is consistently declined** (tuned to ~0), suggesting it is redundant
  with other features.

## Information leakage (important for human-facing play)

The search halts at the turn boundary (`state.current_player != myself` in `expectiminimax`), so
it never simulates the opponent's turn at any depth.

- **`e1` does not leak.** With `max_depth = 1` the recursion depth is 0, so no opponent nodes are
  expanded at all.
- **`e2`/`e3` have a narrow leak surface**: the minimizing branch handles stack-driven opponent
  choices during your own turn (promoting after a knockout, forced discards) and enumerates them
  with the full-information `generate_possible_actions`. There is an existing `TODO` in
  `expectiminimax_player.rs` acknowledging this is unsound.
- The value function itself does not leak: `calculate_active_pokemon_online_score` reads
  `deck + hand`, but that *union* is derivable from public information (decklist minus in-play
  minus discard).

Before shipping `e2`/`e3` as difficulty tiers against human opponents, the minimizing branch needs
a restricted move generator that enumerates only from public information.

## Appendix

### Useful commands

```bash
# Quick strength check against the weak gauntlet
cargo run --release --bin eval_bot -- --candidate e1 --opponents r,w --games 150 --max-decks 10

# Head-to-head between two coefficient sets
cargo run --release --bin eval_bot -- --candidate e1 --params a.json \
  --opponents e1 --opponent-params b.json --games 150 --max-decks 10 --seed 999999

# Machine-readable output for an optimizer loop
--json out.json --fitness-only
```

### Cost estimates

Roughly, on a 2-core machine: `e1` runs ~40-75 games/sec; `e2` is several times slower; `e3` is
~0.5 games/sec (a 32-deck × 3-opponent × 100-game sweep is ~11 hours). Always calibrate with a
tiny run before committing to a long one, and print the plan (`decks × opponents × seats × games`)
before starting.

### Code quality

Run `cargo fmt` and `cargo clippy --all-targets --all-features -- -D warnings` before committing.
