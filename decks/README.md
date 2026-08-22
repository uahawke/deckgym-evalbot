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

**2026-08-22: three current tournament-meta decks added**, sourced from Limitless TCG
(play.limitlesstcg.com/decks?game=pocket -- the top-finishing decklist for each of the
three highest-usage archetypes at the time, cross-checked card-by-card against this
repo's own card database rather than trusted blind):

- `mega-lucario.txt` -- #1 by usage share (9.73%), Fighting
- `vespiquen-shuckle.txt` -- #2 by usage share (7.44%), Grass
- `hoopa-mega-absol.txt` -- #3 by usage share (5.85%) and highest win rate among the top
  3 (53.20%), Darkness

These lean on Mega Evolution and newer `ex` mechanics (sets up through B4) the existing
pool barely touches -- a different kind of coverage gap than the Stage-2 one above, worth
keeping in mind if `train/`'s composition ever gets audited again. Adding them didn't
disturb the first-10-alphabetically validation split (see above); all three sort well
after position 10.
