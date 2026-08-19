# Deck splits

`train/` holds the decks used for coefficient tuning. It is `example_decks/`
minus three held-out decks:

- **flareon**, **donphan** — v4's two weakest matchups, held out to test whether
  tuning gains generalize to archetypes the optimizer never optimized against
- **blastoiseex** — a slow setup deck that earlier coefficient sets sacrificed;
  held out as the hardest generalization case

Validation always runs against the first 10 decks of `example_decks/` with
`--seed 999999`, which includes all three held-out decks. Keep this split stable
so results stay comparable across tuning runs.

**`train/` composition note:** only `hitmonlee.txt` and `mewtwoex.txt` (2 of 29) contain any
Stage-2 Pokemon, and none carry Rare Candy -- the rest are Basic/Stage-1 decks. Coefficients
about evolution-line progress (see the tune-bot skill's Tier B features) get very little training
signal from this pool as a result. That's a real gap if evolution-line features are worth
continuing to invest in, not just an artifact of blastoiseex being held out.
