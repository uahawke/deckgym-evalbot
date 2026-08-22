import { useEffect, useState } from "react";
import { listDecks } from "../api";
import type { DeckChoice, DeckInfo } from "../types";
import { DeckBuilder } from "./DeckBuilder";
import "./DeckSelect.css";

/** One "Your deck"/"AI's deck" picker: a dropdown of curated presets, or -- once built -- a
 * summary of a custom deck with Edit/"use a preset instead" controls. */
function DeckPicker({
  label,
  decks,
  choice,
  onPickPreset,
  onOpenBuilder,
  onClearCustom,
}: {
  label: string;
  decks: DeckInfo[];
  choice: DeckChoice;
  onPickPreset: (path: string) => void;
  onOpenBuilder: () => void;
  onClearCustom: () => void;
}) {
  if ("list" in choice) {
    return (
      <div className="deck-select-field">
        {label}
        <div className="deck-select-custom-summary">
          <span>Custom deck ({choice.summary.card_count} cards)</span>
          <button type="button" onClick={onOpenBuilder}>
            Edit
          </button>
          <button type="button" onClick={onClearCustom}>
            Use a preset instead
          </button>
        </div>
      </div>
    );
  }
  return (
    <div className="deck-select-field">
      {label}
      <select value={choice.path} onChange={(e) => onPickPreset(e.target.value)}>
        {decks.map((d) => (
          <option key={d.path} value={d.path}>
            {d.label}
          </option>
        ))}
      </select>
      <button type="button" className="deck-select-build-link" onClick={onOpenBuilder}>
        Build a custom deck instead
      </button>
    </div>
  );
}

export function DeckSelect({
  onStart,
  loading,
  error,
}: {
  onStart: (deckHuman: DeckChoice, deckAi: DeckChoice, humanSeat: number, aiDepth: number) => void;
  loading: boolean;
  error: string | null;
}) {
  const [decks, setDecks] = useState<DeckInfo[] | null>(null);
  const [decksError, setDecksError] = useState<string | null>(null);
  const [deckHuman, setDeckHuman] = useState<DeckChoice | null>(null);
  const [deckAi, setDeckAi] = useState<DeckChoice | null>(null);
  const [humanSeat, setHumanSeat] = useState(0);
  const [aiDepth, setAiDepth] = useState(2);
  const [building, setBuilding] = useState<"human" | "ai" | null>(null);

  useEffect(() => {
    listDecks()
      .then((d) => {
        setDecks(d);
        setDeckHuman({ path: d[0]?.path ?? "" });
        setDeckAi({ path: d[1]?.path ?? d[0]?.path ?? "" });
      })
      .catch((e) => setDecksError(e instanceof Error ? e.message : String(e)));
  }, []);

  const canStart =
    decks &&
    deckHuman &&
    deckAi &&
    ("list" in deckHuman || deckHuman.path) &&
    ("list" in deckAi || deckAi.path);

  return (
    <div className="deck-select">
      <h1>Play against the AI</h1>
      <p className="deck-select-sub">
        The opponent is <strong>{aiDepth === 3 ? "e3" : "e2"}</strong> (depth-{aiDepth} search),
        tuned via <code>tuned_params_v6.json</code>.
      </p>

      {decksError && <div className="deck-select-error">Failed to load decks: {decksError}</div>}

      {decks && deckHuman && deckAi && (
        <>
          <DeckPicker
            label="Your deck"
            decks={decks}
            choice={deckHuman}
            onPickPreset={(path) => setDeckHuman({ path })}
            onOpenBuilder={() => setBuilding("human")}
            onClearCustom={() => setDeckHuman({ path: decks[0]?.path ?? "" })}
          />

          <DeckPicker
            label="AI's deck"
            decks={decks}
            choice={deckAi}
            onPickPreset={(path) => setDeckAi({ path })}
            onOpenBuilder={() => setBuilding("ai")}
            onClearCustom={() => setDeckAi({ path: decks[1]?.path ?? decks[0]?.path ?? "" })}
          />

          <label className="deck-select-field">
            Go first?
            <select value={humanSeat} onChange={(e) => setHumanSeat(Number(e.target.value))}>
              <option value={0}>You go first</option>
              <option value={1}>AI goes first</option>
            </select>
          </label>

          <label className="deck-select-field">
            Difficulty
            <select value={aiDepth} onChange={(e) => setAiDepth(Number(e.target.value))}>
              <option value={2}>Normal (e2)</option>
              <option value={3}>Hard (e3) -- slower, stronger search</option>
            </select>
          </label>
        </>
      )}

      {error && <div className="deck-select-error">{error}</div>}

      <button
        className="deck-select-start"
        disabled={loading || !canStart}
        onClick={() => canStart && onStart(deckHuman!, deckAi!, humanSeat, aiDepth)}
      >
        {loading ? "Starting..." : decks ? "Start game" : "Loading decks..."}
      </button>

      {building && (
        <DeckBuilder
          initialList={
            building === "human"
              ? deckHuman && "list" in deckHuman
                ? deckHuman.list
                : undefined
              : deckAi && "list" in deckAi
                ? deckAi.list
                : undefined
          }
          onCancel={() => setBuilding(null)}
          onSave={(list, summary) => {
            if (building === "human") setDeckHuman({ list, summary });
            else setDeckAi({ list, summary });
            setBuilding(null);
          }}
        />
      )}
    </div>
  );
}
