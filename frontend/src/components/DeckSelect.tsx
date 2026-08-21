import { useState } from "react";
import "./DeckSelect.css";

// A small curated set for v0. The backend actually accepts any server-local decks-folder path;
// exposing raw file paths to a real player isn't reasonable, so this stands in for a proper
// "list available decks" endpoint/flow later.
const DECKS: { label: string; path: string }[] = [
  { label: "Mewtwo ex", path: "example_decks/mewtwoex.txt" },
  { label: "Blastoise ex", path: "example_decks/blastoiseex.txt" },
  { label: "Fire", path: "example_decks/fire.txt" },
  { label: "Arceus & Dialga", path: "example_decks/arceusdialga.txt" },
  { label: "Altaria", path: "example_decks/altaria.txt" },
  { label: "Weezing / Arbok", path: "example_decks/weezing-arbok.txt" },
];

export function DeckSelect({
  onStart,
  loading,
  error,
}: {
  onStart: (deckHuman: string, deckAi: string, humanSeat: number) => void;
  loading: boolean;
  error: string | null;
}) {
  const [deckHuman, setDeckHuman] = useState(DECKS[0].path);
  const [deckAi, setDeckAi] = useState(DECKS[1].path);
  const [humanSeat, setHumanSeat] = useState(0);

  return (
    <div className="deck-select">
      <h1>Play against the AI</h1>
      <p className="deck-select-sub">
        The opponent is <strong>e2</strong> (depth-2 search), tuned via {" "}
        <code>tuned_params_v6.json</code>.
      </p>

      <label className="deck-select-field">
        Your deck
        <select value={deckHuman} onChange={(e) => setDeckHuman(e.target.value)}>
          {DECKS.map((d) => (
            <option key={d.path} value={d.path}>
              {d.label}
            </option>
          ))}
        </select>
      </label>

      <label className="deck-select-field">
        AI's deck
        <select value={deckAi} onChange={(e) => setDeckAi(e.target.value)}>
          {DECKS.map((d) => (
            <option key={d.path} value={d.path}>
              {d.label}
            </option>
          ))}
        </select>
      </label>

      <label className="deck-select-field">
        Go first?
        <select value={humanSeat} onChange={(e) => setHumanSeat(Number(e.target.value))}>
          <option value={0}>You go first</option>
          <option value={1}>AI goes first</option>
        </select>
      </label>

      {error && <div className="deck-select-error">{error}</div>}

      <button
        className="deck-select-start"
        disabled={loading}
        onClick={() => onStart(deckHuman, deckAi, humanSeat)}
      >
        {loading ? "Starting..." : "Start game"}
      </button>
    </div>
  );
}
