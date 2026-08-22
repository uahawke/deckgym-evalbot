import { useEffect, useMemo, useState } from "react";
import { listCards, validateDeck } from "../api";
import type { Card, DeckSummary, EnergyType } from "../types";
import { cardName } from "../types";
import { CardView } from "./CardView";
import "./DeckBuilder.css";

const MAX_RESULTS = 60;
const MAX_COPIES = 2;
const DECK_SIZE = 20;

const ENERGY_TYPES: EnergyType[] = [
  "Grass",
  "Fire",
  "Water",
  "Lightning",
  "Psychic",
  "Fighting",
  "Darkness",
  "Metal",
  "Dragon",
  "Colorless",
];

function cardId(card: Card): string {
  return "Pokemon" in card ? card.Pokemon.id : card.Trainer.id;
}

/** Parses a saved decklist (the "<count> <card id>" text format) back into per-id counts, so
 * re-opening the builder to edit a previously-built deck starts from where it left off. */
function parseCounts(list: string): Record<string, number> {
  const counts: Record<string, number> = {};
  for (const line of list.split("\n")) {
    const trimmed = line.trim();
    const spaceIdx = trimmed.indexOf(" ");
    if (spaceIdx === -1) continue;
    const count = Number(trimmed.slice(0, spaceIdx));
    const id = trimmed.slice(spaceIdx + 1).trim();
    if (Number.isFinite(count) && count > 0) counts[id] = count;
  }
  return counts;
}

/** A visual deck builder: search/filter the full card database, click to add up to 2 copies of
 * a card, and validate against the same legality rules (`Deck::is_valid`) the server enforces --
 * exactly 20 cards, at least 1 Basic, at most 2 copies of any card name, selectable energy. */
export function DeckBuilder({
  initialList,
  onSave,
  onCancel,
}: {
  initialList?: string;
  onSave: (list: string, summary: DeckSummary) => void;
  onCancel: () => void;
}) {
  const [cards, setCards] = useState<Card[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [kindFilter, setKindFilter] = useState<"All" | "Pokemon" | "Trainer">("All");
  const [energyFilter, setEnergyFilter] = useState<EnergyType | "All">("All");
  const [counts, setCounts] = useState<Record<string, number>>(() =>
    initialList ? parseCounts(initialList) : {},
  );
  const [validation, setValidation] = useState<DeckSummary | null>(null);
  const [validationError, setValidationError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    listCards()
      .then(setCards)
      .catch((e) => setLoadError(e instanceof Error ? e.message : String(e)));
  }, []);

  const cardsById = useMemo(() => {
    const m = new Map<string, Card>();
    cards?.forEach((c) => m.set(cardId(c), c));
    return m;
  }, [cards]);

  const totalCount = Object.values(counts).reduce((a, b) => a + b, 0);

  const filtered = useMemo(() => {
    if (!cards) return [];
    const q = search.trim().toLowerCase();
    return cards.filter((c) => {
      const isPokemon = "Pokemon" in c;
      if (kindFilter === "Pokemon" && !isPokemon) return false;
      if (kindFilter === "Trainer" && isPokemon) return false;
      if (energyFilter !== "All" && (!isPokemon || c.Pokemon.energy_type !== energyFilter)) {
        return false;
      }
      return !q || cardName(c).toLowerCase().includes(q);
    });
  }, [cards, search, kindFilter, energyFilter]);

  const shown = filtered.slice(0, MAX_RESULTS);

  function adjust(id: string, delta: number) {
    setCounts((prev) => {
      const next = { ...prev };
      const updated = Math.max(0, Math.min(MAX_COPIES, (next[id] ?? 0) + delta));
      if (updated === 0) delete next[id];
      else next[id] = updated;
      return next;
    });
    setValidation(null);
    setValidationError(null);
  }

  /** Explicit `Energy:` line, computed from the selected Pokémon's own types -- letting the
   * server derive it (its fallback for a decklist with no such line) would include every type
   * verbatim, and a Colorless- or Dragon-type Pokémon's own type isn't a type the Energy Zone can
   * actually generate (`EnergyType::is_selectable`); a deck with any such Pokémon would otherwise
   * fail legality even though real decks built around them declare *other* energy types instead. */
  function energyLine(): string {
    const types = new Set<EnergyType>();
    for (const id of Object.keys(counts)) {
      const card = cardsById.get(id);
      const type = card && "Pokemon" in card ? card.Pokemon.energy_type : null;
      if (type && type !== "Colorless" && type !== "Dragon") types.add(type);
    }
    return types.size > 0 ? `Energy: ${[...types].join(",")}\n` : "";
  }

  function buildListText(): string {
    const cardLines = Object.entries(counts)
      .map(([id, count]) => `${count} ${id}`)
      .join("\n");
    return energyLine() + cardLines;
  }

  async function check(onValid?: (list: string, summary: DeckSummary) => void) {
    setBusy(true);
    setValidationError(null);
    try {
      const list = buildListText();
      const summary = await validateDeck(list);
      setValidation(summary);
      onValid?.(list, summary);
    } catch (e) {
      setValidation(null);
      setValidationError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="deck-builder-backdrop">
      <div className="deck-builder">
        <div className="deck-builder-header">
          <h2>Build a deck</h2>
          <button className="deck-builder-close" onClick={onCancel}>
            ✕
          </button>
        </div>

        {loadError && <div className="deck-builder-error">Failed to load cards: {loadError}</div>}

        <div className="deck-builder-body">
          <div className="deck-builder-search-pane">
            <div className="deck-builder-search">
              <input
                type="text"
                placeholder="Search cards by name..."
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />
              <select
                value={kindFilter}
                onChange={(e) => setKindFilter(e.target.value as "All" | "Pokemon" | "Trainer")}
              >
                <option value="All">All cards</option>
                <option value="Pokemon">Pokémon</option>
                <option value="Trainer">Trainer</option>
              </select>
              <select
                value={energyFilter}
                onChange={(e) => setEnergyFilter(e.target.value as EnergyType | "All")}
                disabled={kindFilter === "Trainer"}
              >
                <option value="All">Any energy</option>
                {ENERGY_TYPES.map((t) => (
                  <option key={t} value={t}>
                    {t}
                  </option>
                ))}
              </select>
            </div>

            <div className="deck-builder-results">
              {cards === null && !loadError && (
                <div className="deck-builder-loading">Loading cards...</div>
              )}
              {shown.map((c) => {
                const id = cardId(c);
                const count = counts[id] ?? 0;
                return (
                  <div key={id} className="deck-builder-card">
                    <CardView card={c} small />
                    <div className="deck-builder-card-controls">
                      <button onClick={() => adjust(id, -1)} disabled={count === 0}>
                        −
                      </button>
                      <span>{count}</span>
                      <button onClick={() => adjust(id, 1)} disabled={count >= MAX_COPIES}>
                        +
                      </button>
                    </div>
                  </div>
                );
              })}
              {filtered.length > MAX_RESULTS && (
                <div className="deck-builder-more">
                  Showing {MAX_RESULTS} of {filtered.length} matches -- refine your search to see
                  more.
                </div>
              )}
              {cards && filtered.length === 0 && (
                <div className="deck-builder-empty">No cards match.</div>
              )}
            </div>
          </div>

          <div className="deck-builder-deck-pane">
            <div className="deck-builder-deck-title">
              Your deck: {totalCount} / {DECK_SIZE}
            </div>
            <div className="deck-builder-deck-list">
              {Object.entries(counts).length === 0 && (
                <div className="deck-builder-empty">No cards selected yet.</div>
              )}
              {Object.entries(counts).map(([id, count]) => {
                const card = cardsById.get(id);
                return (
                  <div key={id} className="deck-builder-deck-row">
                    <span className="deck-builder-deck-count">{count}x</span>
                    <span className="deck-builder-deck-name">{card ? cardName(card) : id}</span>
                    <button className="deck-builder-deck-remove" onClick={() => adjust(id, -count)}>
                      ✕
                    </button>
                  </div>
                );
              })}
            </div>

            {validation && (
              <div className="deck-builder-valid">
                Legal deck -- energy: {validation.energy_types.join(", ")}
              </div>
            )}
            {validationError && <div className="deck-builder-error">{validationError}</div>}

            <div className="deck-builder-actions">
              <button onClick={() => check()} disabled={busy || totalCount === 0}>
                Check legality
              </button>
              <button onClick={onCancel}>Cancel</button>
              <button
                className="deck-builder-save"
                onClick={() => check(onSave)}
                disabled={busy || totalCount === 0}
              >
                {busy ? "Checking..." : "Save deck"}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
