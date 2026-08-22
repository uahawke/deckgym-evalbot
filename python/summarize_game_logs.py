#!/usr/bin/env python3
"""Summarizes archived game logs (game_logs/*.json, written by the web server -- see
GAME_LOGS_DIR's doc comment in src/web/mod.rs) into a report a human can use to decide
which decks are worth promoting into decks/train/example_decks/ for a future CMA-ES
tuning gauntlet.

Purely a read-only report: it doesn't touch decks/train, doesn't run any tuning, and
nothing here is automatic -- that promotion decision is still a manual, human call (see
the tune-bot skill and GAME_LOGS_DIR's doc comment for why: CMA-ES tunes by playing new
games with candidate coefficients, not by fitting historical transcripts, so this is
gauntlet-diversity fodder, not training data in its own right).

Usage:
    python3 python/summarize_game_logs.py
    python3 python/summarize_game_logs.py --min-games 3
    python3 python/summarize_game_logs.py --game-logs-dir /path/to/game_logs --json
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import defaultdict
from pathlib import Path


def deck_key(source: dict) -> tuple[str, str]:
    """Groups decks by identity: a curated deck by its file path, a player-submitted one
    by its exact decklist text (so two players who build the same 20 cards count as the
    same deck, but two different builds never collide)."""
    if "Path" in source:
        return ("preset", source["Path"])
    return ("custom", source["List"])


def deck_label(kind: str, identity: str) -> str:
    """A short human-readable label -- the path for a preset, or a card-id preview for a
    custom decklist (never printed in full; that's just noise in a summary table)."""
    if kind == "preset":
        return identity
    card_lines = [
        line.strip() for line in identity.splitlines() if line.strip() and not line.startswith("Energy:")
    ]
    preview = ", ".join(card_lines[:3])
    more = f" (+{len(card_lines) - 3} more)" if len(card_lines) > 3 else ""
    return f"custom [{preview}{more}]"


def load_records(game_logs_dir: Path) -> list[dict]:
    records = []
    for path in sorted(game_logs_dir.glob("*.json")):
        try:
            records.append(json.loads(path.read_text()))
        except (json.JSONDecodeError, OSError) as e:
            print(f"skipping {path.name}: {e}", file=sys.stderr)
    return records


def summarize(records: list[dict]) -> list[dict]:
    """One row per distinct deck (see deck_key), aggregated across every game it appeared
    in on either side -- a deck's identity doesn't change depending on which seat played it."""
    stats: dict[tuple[str, str], dict] = defaultdict(
        lambda: {"games": 0, "wins": 0, "losses": 0, "ties": 0}
    )
    labels: dict[tuple[str, str], str] = {}

    for record in records:
        winner = record["winner"]
        sides = [
            (record["deck_human"], record["human_seat"]),
            (record["deck_ai"], (record["human_seat"] + 1) % 2),
        ]
        for deck_source, seat in sides:
            key = deck_key(deck_source)
            labels.setdefault(key, deck_label(*key))
            row = stats[key]
            row["games"] += 1
            if winner == "Tie":
                row["ties"] += 1
            elif winner["Win"] == seat:
                row["wins"] += 1
            else:
                row["losses"] += 1

    rows = []
    for key, row in stats.items():
        kind, _ = key
        rows.append(
            {
                "kind": kind,
                "label": labels[key],
                "games": row["games"],
                "wins": row["wins"],
                "losses": row["losses"],
                "ties": row["ties"],
                "win_rate": row["wins"] / row["games"] if row["games"] else 0.0,
            }
        )
    rows.sort(key=lambda r: r["games"], reverse=True)
    return rows


def print_table(records: list[dict], rows: list[dict]) -> None:
    print(f"{len(records)} archived game(s), {len(rows)} distinct deck(s)\n")
    if not rows:
        return
    print(f"{'Games':>6} {'Wins':>5} {'Losses':>7} {'Ties':>5} {'Win%':>6}  Deck")
    print("-" * 100)
    for r in rows:
        print(
            f"{r['games']:>6} {r['wins']:>5} {r['losses']:>7} {r['ties']:>5} "
            f"{r['win_rate'] * 100:>5.1f}%  {r['label']}"
        )

    custom = [r for r in rows if r["kind"] == "custom"]
    if custom:
        print(
            f"\n{len(custom)} player-submitted deck(s) seen. Review these for promotion into "
            "decks/train/example_decks/ -- nothing here does that automatically."
        )


def main() -> None:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "--game-logs-dir",
        type=Path,
        default=Path("game_logs"),
        help="Directory of archived game_logs/*.json records (default: ./game_logs)",
    )
    parser.add_argument(
        "--min-games",
        type=int,
        default=1,
        help="Only show decks that appeared in at least this many games (default: 1)",
    )
    parser.add_argument(
        "--json", action="store_true", help="Print a machine-readable JSON report instead of a table"
    )
    args = parser.parse_args()

    if not args.game_logs_dir.exists():
        print(f"{args.game_logs_dir} does not exist -- no games archived yet.")
        return

    records = load_records(args.game_logs_dir)
    if not records:
        print(f"No game logs found in {args.game_logs_dir}.")
        return

    rows = [r for r in summarize(records) if r["games"] >= args.min_games]

    if args.json:
        print(json.dumps({"total_games": len(records), "decks": rows}, indent=2))
    else:
        print_table(records, rows)


if __name__ == "__main__":
    main()
