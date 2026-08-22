# deckgym-evalbot

A fork of [bcollazo/deckgym-core](https://github.com/bcollazo/deckgym-core),
a Pokémon TCG Pocket simulation engine, focused on measuring and improving
AI player strength.

## What this adds

- **`eval_bot`** — gauntlet harness with Wilson confidence intervals,
  seat-swapping, and head-to-head comparison between coefficient sets
- **`python/tune_value_function.py`** — CMA-ES tuner treating the simulator
  as a black-box fitness function, with per-generation deck rotation, optional
  seeding from an existing champion (`--init-params`), and a mid-run held-out
  probe that tracks a best-by-generalization checkpoint independently of
  training fitness and can stop a run early if it stalls
- **Tactical and evolution-line features** in the parametric value function
  (weakness matchup, knockout availability, hand playability, evolution
  readiness)
- **`.claude/skills/tune-bot/`** — methodology notes, including the
  measurement pitfalls that produce confidently wrong results
- **`python/summarize_game_logs.py`** — reports deck frequency/win-rate from the web
  server's `game_logs/` archive, to help decide which player-submitted decks are worth
  promoting into `decks/train`/`example_decks/` for a future tuning gauntlet

## Usage

```bash
# Compare two coefficient sets head-to-head
cargo run --release --bin eval_bot -- \
  --candidate e1 --params tuned_params_v6.json \
  --opponents e1 --games 150 --max-decks 10 --seed 999999

# Tune against a fixed reference bot
python3 -m venv .venv && source .venv/bin/activate
pip install cma numpy
python -u python/tune_value_function.py \
  --generations 20 --popsize 12 --games 50 --decks 12 \
  --decks-folder decks/train \
  --opponents e1 --opponent-params tuned_params_v6.json \
  --init-params tuned_params_v6.json \
  --out tuned_params_v7.json

# Review which decks players actually used against the web opponent
python3 python/summarize_game_logs.py
```

## Results

Current champion `tuned_params_v6.json` wins **63%** head-to-head against the
hand-tuned baseline at search depth 1, over 3,000 games — the same margin as
its predecessor `tuned_params_v5.json`, since v6 is v5 plus two evolution-line
coefficients (`playable_hand_size`, `bench_evolution_potential`) that
validated consistently across four independent tuning runs. It does **not**
decisively beat v5 head-to-head (50.24% [48.98%, 51.51%] over 6,000 games on
an unseen seed — statistically indistinguishable), but it wins on consistency:
every deck in the 10-deck validation set lands in a 46-56% win-rate band
*against v5*, versus the much wider, more exploitable spread that candidate
coefficient sets produced against v5 throughout this round of tuning.

One honest caveat: this does not mean `blastoiseex` (the deck whose weak
matchup motivated this round of tuning) is now a strong matchup in absolute
terms — against the plain hand-tuned baseline it's still weak (42.8%,
essentially unchanged from v5's 43.2%). What improved is narrower: earlier
tuning attempts this round routinely lost decisively to v5 specifically on
`blastoiseex` (down to 31-35% in some runs); v6 is the first candidate to
reach parity with v5 there (48.4% on an unseen seed) instead of losing ground
to it. See `.claude/skills/tune-bot/SKILL.md`'s Known findings for the full
methodology history behind this number.

Note that value function parameters are **depth-specific** and do not transfer
to `e2`/`e3` — deeper search evaluates a different distribution of states.

## License

AGPL-3.0, inherited from the upstream project. See `LICENSE.txt`.
