"""CMA-ES tuner for ValueFunctionParams.

Treats the Rust simulator as a black-box fitness function: writes candidate coefficients
to JSON, shells out to `eval_bot`, reads the win rate back. No FFI, no bindings.

    pip install cma numpy
    python python/tune_value_function.py --generations 50 --games 50 --decks 4

Design notes:

* **Scale normalization.** The raw coefficients span five orders of magnitude (points=10_000,
  opponent_discard_size=0.1). CMA-ES uses one step size for all dimensions, so optimizing raw
  values would take absurd steps in the small dimensions and negligible ones in the large.
  We optimize a normalized vector z where param_i = baseline_i + z_i * scale_i.

* **Frozen dimensions.** `is_winner` isn't a tradeoff -- it's a sentinel that makes terminal
  wins dominate everything else. Perturbing it just breaks the ordering. Frozen by default.

* **Ties are not half-wins.** The engine declares a tie at turn 30, and ~20% of games against
  weak opponents time out. Scoring those at 0.5 rewards stalling, which is a local optimum a
  tuner will happily find. Default fitness is wins/(wins+losses) with an explicit tie penalty.

* **Seed rotation.** Holding seeds fixed makes comparisons paired (much less noise) but invites
  overfitting to specific shuffles. We rotate every --seed-rotation generations and validate the
  final result on seeds never used during the search.

* **Deck rotation.** eval_bot's --max-decks truncates its --decks-folder listing to a fixed
  alphabetical prefix, so a fixed --decks count sampled the *same* subset every generation --
  CMA-ES could memorize answers to a fixed deck quiz instead of generalizing. Every generation we
  symlink a fresh random --decks-sized sample from --decks-folder into a scratch dir and point
  eval_bot at that instead, so the full pool gets sampled over the course of a run.
"""

import argparse
import json
import os
import random
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

import numpy as np

try:
    import cma
except ImportError:
    sys.exit("Missing dependency. Run: pip install cma numpy")

FIELD_NAMES = [
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
    "playable_hand_size",
    "evolution_readiness",
    "bench_evolution_potential",
]

BASELINE = {
    "points": 10_000.0,
    "pokemon_value": 1.0,
    "hand_size": 1.0,
    "deck_size": 1.0,
    "active_retreat_cost": 1.0,
    "active_pokemon_online_score": 500.0,
    "active_safety": 1.0,
    "active_has_tool": 10.0,
    "is_winner": 100_000.0,
    "turns_until_opponent_wins": 100.0,
    "online_pokemon_count": 0.0,
    "energy_distance_to_online": 0.0,
    "opponent_discard_size": 0.1,
    # New features start disabled so the baseline is identical to the old one and any gain is
    # attributable to the features rather than a shifted starting point.
    "active_weakness_matchup": 0.0,
    "can_ko_opponent_active": 0.0,
    "opponent_can_ko_my_active": 0.0,
    # playable_hand_size and bench_evolution_potential are pinned here (not 0.0) and frozen
    # below: four independent runs on 2026-08-18 (one local smoke test, three parallel CI seed
    # replicates) agreed on sign for both -- playable_hand_size positive (+8 to +25),
    # bench_evolution_potential negative (-41 to -369) -- so we fix them near their observed
    # average and let CMA-ES spend its budget on the dimensions that are still unsettled
    # (evolution_readiness flipped sign between runs and is NOT frozen).
    "playable_hand_size": 17.0,
    "evolution_readiness": 0.0,
    "bench_evolution_potential": -174.0,
}

# Step scale per dimension. For zero-valued baselines there is no magnitude to infer, so we
# supply a plausible one -- these two features are currently disabled and the tuner's job is
# partly to discover whether they are worth switching on.
SCALES = {name: (abs(v) if v != 0.0 else 1.0) for name, v in BASELINE.items()}
SCALES["online_pokemon_count"] = 50.0
SCALES["energy_distance_to_online"] = 50.0
# 0/+-1 indicators: the coefficient is directly "how much board value is this worth".
SCALES["active_weakness_matchup"] = 200.0
SCALES["can_ko_opponent_active"] = 500.0
SCALES["opponent_can_ko_my_active"] = 500.0
# Tier B: evolution-line features, also disabled at baseline. playable_hand_size and
# bench_evolution_potential are small integer counts (differenced, so roughly -5..5) like
# hand_size; evolution_readiness is a 0/1 indicator on par with the Tier A tactical features.
SCALES["playable_hand_size"] = 10.0
SCALES["evolution_readiness"] = 300.0
SCALES["bench_evolution_potential"] = 100.0

FROZEN = {"is_winner", "playable_hand_size", "bench_evolution_potential"}


def z_to_params(z, free_names):
    """Map a normalized CMA-ES vector back to full coefficients."""
    params = dict(BASELINE)
    for value, name in zip(z, free_names):
        params[name] = BASELINE[name] + value * SCALES[name]
    return params


def rotate_deck_folder(decks_folder, decks_per_eval, rng, previous=None):
    """Symlink a fresh random sample of decks_per_eval decklists from decks_folder into a new
    scratch dir, and remove the previous one. Every eval this generation shares the sample (fair
    comparison across the population); the next generation draws a new one."""
    if previous is not None:
        shutil.rmtree(previous, ignore_errors=True)

    pool = sorted(Path(decks_folder).glob("*.txt"))
    if not pool:
        sys.exit(f"No .txt decks found in {decks_folder}")
    k = len(pool) if decks_per_eval <= 0 else min(decks_per_eval, len(pool))
    sample = rng.sample(pool, k)

    scratch = tempfile.mkdtemp(prefix="deck_rotation_")
    for deck_path in sample:
        os.symlink(deck_path.resolve(), Path(scratch) / deck_path.name)
    return scratch


def evaluate(params, args, seed, decks_folder):
    """Run the gauntlet for one candidate and return (fitness, raw_report)."""
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as pf:
        json.dump(params, pf)
        params_path = pf.name
    report_path = params_path.replace(".json", ".report.json")

    cmd = [
        "cargo", "run", "--release", "--quiet", "--bin", "eval_bot", "--",
        "--candidate", args.candidate,
        "--params", params_path,
        "--opponents", args.opponents,
        "--games", str(args.games),
        "--max-decks", "0",  # decks_folder is already exactly the rotated sample
        "--decks-folder", decks_folder,
        "--seed", str(seed),
    ] + ([] if not args.opponent_params else [
        "--opponent-params", args.opponent_params,
    ]) + [
        "--json", report_path,
        "--fitness-only",
    ]
    try:
        subprocess.run(cmd, check=True, capture_output=True, text=True)
        with open(report_path) as f:
            report = json.load(f)
    except subprocess.CalledProcessError as err:
        print(f"  eval_bot failed: {err.stderr[:400]}")
        return 0.0, None
    finally:
        for path in (params_path, report_path):
            if os.path.exists(path):
                os.remove(path)

    wins = report["candidate_wins"]
    losses = report["opponent_wins"]
    ties = report["ties"]
    decided = wins + losses
    if decided == 0:
        return 0.0, report
    # Decisive win rate, minus a penalty for stalling into the turn limit.
    fitness = wins / decided - args.tie_penalty * (ties / max(1, wins + losses + ties))
    return fitness, report


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--generations", type=int, default=50)
    p.add_argument("--popsize", type=int, default=12)
    p.add_argument("--sigma", type=float, default=0.5)
    p.add_argument("--games", type=int, default=50, help="games per (deck, opponent, seat) cell")
    p.add_argument("--decks", type=int, default=4)
    p.add_argument("--decks-folder", default="example_decks",
                   help="folder of decklists to tune against")
    p.add_argument("--candidate", default="e1", help="e1 = one-ply value-function player; only e<n> candidates actually consume --params")
    p.add_argument("--opponents", default="w", help="use 'e1' for head-to-head tuning")
    p.add_argument("--opponent-params", default=None,
                   help="JSON params for the opponent. With --opponents e1 this makes fitness a "
                        "direct head-to-head win rate against a fixed reference bot, which has "
                        "full dynamic range instead of saturating near 90% vs weak bots.")
    p.add_argument("--seed", type=int, default=1)
    p.add_argument("--seed-rotation", type=int, default=5, help="0 disables rotation")
    p.add_argument("--tie-penalty", type=float, default=0.25)
    p.add_argument("--out", default="tuned_params.json")
    args = p.parse_args()

    free_names = [n for n in FIELD_NAMES if n not in FROZEN]
    print(f"Optimizing {len(free_names)} of {len(FIELD_NAMES)} coefficients "
          f"(frozen: {sorted(FROZEN)})")

    deck_rng = random.Random(args.seed)
    deck_folder = rotate_deck_folder(args.decks_folder, args.decks, deck_rng)
    print(f"  [decks: {', '.join(dp.name for dp in sorted(Path(deck_folder).iterdir()))}]")

    baseline_fitness, baseline_report = evaluate(dict(BASELINE), args, args.seed, deck_folder)
    if baseline_report:
        print(f"Baseline fitness: {baseline_fitness:.4f} "
              f"(win rate {baseline_report['win_rate']:.3f}, "
              f"{baseline_report['ties']} ties / {baseline_report['total_games']} games)")

    es = cma.CMAEvolutionStrategy(np.zeros(len(free_names)), args.sigma,
                                  {"popsize": args.popsize, "verbose": -9})

    best_fitness, best_params = baseline_fitness, dict(BASELINE)
    seed = args.seed
    start = time.time()

    try:
        for gen in range(args.generations):
            deck_folder = rotate_deck_folder(args.decks_folder, args.decks, deck_rng, deck_folder)

            if args.seed_rotation and gen > 0 and gen % args.seed_rotation == 0:
                seed += 1000
                # Re-score the incumbent on the new seeds so comparisons stay honest.
                best_fitness, _ = evaluate(best_params, args, seed, deck_folder)
                print(f"  [seed rotated to {seed}; incumbent re-scored at {best_fitness:.4f}]")

            solutions = es.ask()
            fitnesses = []
            for z in solutions:
                fit, _ = evaluate(z_to_params(z, free_names), args, seed, deck_folder)
                fitnesses.append(-fit)  # cma minimizes

            es.tell(solutions, fitnesses)

            gen_best_idx = int(np.argmin(fitnesses))
            gen_best_fit = -fitnesses[gen_best_idx]
            if gen_best_fit > best_fitness:
                best_fitness = gen_best_fit
                best_params = z_to_params(solutions[gen_best_idx], free_names)
                with open(args.out, "w") as f:
                    json.dump(best_params, f, indent=2)
                marker = " *saved*"
            else:
                marker = ""

            elapsed = time.time() - start
            print(f"gen {gen+1:3d}/{args.generations}  best={gen_best_fit:.4f}  "
                  f"incumbent={best_fitness:.4f}  ({elapsed/60:.1f} min){marker}")
    finally:
        shutil.rmtree(deck_folder, ignore_errors=True)

    # Write unconditionally: the loop only writes args.out when a generation beats the prior
    # incumbent, so a run that never improves on the baseline would otherwise leave this message
    # claiming a file that was never created.
    with open(args.out, "w") as f:
        json.dump(best_params, f, indent=2)
    print(f"\nBest fitness {best_fitness:.4f} written to {args.out}")
    print("Validate on unseen seeds and more decks before trusting it, e.g.:")
    print(f"  cargo run --release --bin eval_bot -- --candidate e1 --params {args.out} "
          f"--opponents r,w --games 200 --max-decks 8 --seed 999999")


if __name__ == "__main__":
    main()
