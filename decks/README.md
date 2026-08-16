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
