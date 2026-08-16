# deckgym-evalbot

A fork of [bcollazo/deckgym-core](https://github.com/bcollazo/deckgym-core),
a Pokémon TCG Pocket simulation engine, focused on measuring and improving
AI player strength.

## What this adds

- **`eval_bot`** — gauntlet harness with Wilson confidence intervals,
  seat-swapping, and head-to-head comparison between coefficient sets
- **`python/tune_value_function.py`** — CMA-ES tuner treating the simulator
  as a black-box fitness function
- **Tactical features** in the parametric value function (weakness matchup,
  knockout availability)
- **`.claude/skills/tune-bot/`** — methodology notes, including the
  measurement pitfalls that produce confidently wrong results

## Usage

```bash
# Compare two coefficient sets head-to-head
cargo run --release --bin eval_bot -- \
  --candidate e1 --params tuned_params_v5.json \
  --opponents e1 --games 150 --max-decks 10 --seed 999999

# Tune against a fixed reference bot
python3 -m venv .venv && source .venv/bin/activate
pip install cma numpy
python -u python/tune_value_function.py \
  --generations 20 --popsize 12 --games 50 --decks 12 \
  --opponents e1 --opponent-params tuned_params_v5.json \
  --out tuned_params_v6.json
```

## Results

Tuned coefficients (`tuned_params_v5.json`) win 63% head-to-head against the
hand-tuned baseline at search depth 1, over 3,000 games.

Note that value function parameters are **depth-specific** and do not transfer
to `e2`/`e3` — deeper search evaluates a different distribution of states.

## License

AGPL-3.0, inherited from the upstream project. See `LICENSE.txt`.
