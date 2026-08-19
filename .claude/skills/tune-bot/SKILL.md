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
  --init-params <champion>.json \
  --out tuned_params_vN.json
```

- Use `--candidate e1` (the default). `e2`/`e3` are 10-100x slower and infeasible in a loop.
- Hold decks out of the training folder so validation is honest.
- Use `nohup ... &` with `python -u` for long runs; without `-u`, stdout block-buffers and the log
  stays empty for hours.
- New coefficients should default to `0.0` so the baseline is numerically unchanged and any gain
  is attributable to the feature rather than a moved starting point.
- **`--decks` is a per-generation sample size, not a fixed subset.** A fresh random sample is
  drawn from `--decks-folder` every generation (deterministic from `--seed`), so an 18-generation
  run at `--decks 12` covers most or all of a 29-deck pool over the course of a run instead of
  memorizing one fixed subset. The incumbent is re-scored on each new sample before comparing, so
  generation-to-generation comparisons stay honest -- if you add any other kind of rotation,
  re-score the incumbent on the new condition too, or `best > candidate` comparisons become
  apples-to-oranges.
- **`--init-params <file>` seeds the search from an existing champion's coefficients** instead of
  re-discovering BASELINE from scratch every run. It gives a much better *starting* fitness, but
  did not reliably improve *final* validation quality after 18 generations in practice --
  seed-to-seed variance dominates regardless of starting point (see Known findings).
- **The tuner probes held-out decks mid-run and can stop early.** Every `--probe-interval`
  generations it evaluates the training incumbent against whatever's in `--probe-source-folder`
  (default `example_decks`) but NOT in `--decks-folder` -- e.g. the three decks `decks/train`
  excludes -- on a fixed `--probe-seed` so probe-to-probe comparisons stay paired. This never
  feeds into CMA-ES's own objective (`es.tell()` never sees it); it only tracks a separate
  best-by-probe checkpoint, written to `--probe-out` (default: `--out` with `.probe_best`
  inserted), and stops the run after `--probe-patience` consecutive checks with no improvement
  (`0` disables early stopping but keeps probing/reporting). **Prefer the `.probe_best.json`
  output over the plain `--out` file** when both exist -- see Known findings for why training
  fitness alone isn't trustworthy. `--probe-games 0` disables probing entirely.

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
- **Tier B (evolution-line) signs: two of three settled.** `playable_hand_size` (positive) and
  `bench_evolution_potential` (negative) agree in sign across every independent run tried, so
  they're frozen at their observed average rather than re-discovered each time.
  `evolution_readiness`'s sign flips between runs and remains unsettled -- leave it free.
- **The training pool has almost no Stage-2 evolution content.** Only 2 of 29 decks in
  `decks/train` (`hitmonlee.txt`, `mewtwoex.txt`) contain any Stage-2 Pokemon, and none carry
  Rare Candy. Features about evolution-line progress get very little training signal from the
  standard pool -- plausibly why Tier B validates in the right direction (see above) without
  closing the blastoiseex gap (below). A deck pool built for a specific feature set should
  actually contain the situations that feature is meant to recognize.
- **CMA-ES seed-to-seed variance dominates over starting point.** Seeding the search from
  `tuned_params_v5.json` (via `--init-params`) gives a much better starting fitness (~0.47-0.51
  vs. BASELINE's ~0.37-0.40) but final validation quality after 18 generations was statistically
  indistinguishable with and without seeding across three-seed replicate runs -- and which seed
  ends up best/worst flips between runs. At `--generations 18 --popsize 12`, the search does not
  reliably converge to the same quality region regardless of where it starts.
- **`can_ko_opponent_active`'s negative sign is the load-bearing one; `active_weakness_matchup`'s
  isn't.** A paired sign-flip test (negating both together) lost decisively to the un-flipped
  version (43.75% vs. v5 on the blastoiseex mirror). Following up with an isolated flip of
  `active_weakness_matchup` alone, on the standard 10-deck set, landed at 49.48% [47.70%, 51.27%]
  against v5 -- statistically indistinguishable, not a loss. So the paired test's decisive loss is
  attributable to `can_ko_opponent_active`, which despite reading backwards from its own doc
  comment is genuinely net beneficial as tuned. `active_weakness_matchup`'s sign, on its own,
  barely matters either way -- treat it as close to inert rather than settled in either direction.
- **blastoiseex is still unresolved as an outright win**, across every approach tried so far
  (general retuning, Tier B features, deck rotation, v5-seeded search): no checkpoint has beaten
  50% on it across *all* seeds in a run. But **selecting by the held-out probe instead of
  training fitness is a real, validated improvement**, not just plumbing: in a 36-generation
  v5-seeded run with the probe active, the probe-best checkpoint beat the training-best checkpoint
  on blastoiseex in all 3 seeds (35.5/34.3/37.5% -> 52.7/47.7/46.8%, average 35.8% -> 49.1%) and on
  overall validation in all 3 seeds too (two of three landing at "statistically indistinguishable
  from v5" rather than "significantly weaker"). Prefer probe-best checkpoints over training-best
  ones going forward -- see the probe note under "Running the tuner".

## Open threads

Not yet tried, roughly in order of expected leverage given the findings above:

- **More games per cell.** 50 games/cell is noisy at near-50% win rates; CMA-ES may be chasing
  fitness-estimate noise as much as real signal, independent of starting point or deck sampling.
- **More generations: inconclusive so far, confounded with the probe.** A `--generations 36` run
  (double the usual 18) with the probe active let 2 of 3 seeds stop early at 15 and 18 -- right
  around the old budget -- so it doesn't cleanly test whether more generations help on their own;
  the probe-vs-training-fitness selection effect (above) dominated the result. Worth an isolated
  test with `--probe-patience 0` (probe still reports, just can't cut the run short) to see
  whether the extra generations do anything once that confound is removed.
- **A deck pool that actually contains Stage-2/slow-evolution archetypes**, if evolution-line
  features are worth continuing to invest in -- the current pool can't teach what it doesn't
  contain. Note this needs care: adding more such decks to `decks/train` changes what "the training
  pool" means for every finding above that references it.

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
