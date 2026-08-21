import { useEffect, useState } from "react";
import { listDecks } from "../api";
import type { DeckInfo } from "../types";
import "./DeckSelect.css";

export function DeckSelect({
  onStart,
  loading,
  error,
}: {
  onStart: (deckHuman: string, deckAi: string, humanSeat: number) => void;
  loading: boolean;
  error: string | null;
}) {
  const [decks, setDecks] = useState<DeckInfo[] | null>(null);
  const [decksError, setDecksError] = useState<string | null>(null);
  const [deckHuman, setDeckHuman] = useState("");
  const [deckAi, setDeckAi] = useState("");
  const [humanSeat, setHumanSeat] = useState(0);

  useEffect(() => {
    listDecks()
      .then((d) => {
        setDecks(d);
        setDeckHuman(d[0]?.path ?? "");
        setDeckAi(d[1]?.path ?? d[0]?.path ?? "");
      })
      .catch((e) => setDecksError(e instanceof Error ? e.message : String(e)));
  }, []);

  return (
    <div className="deck-select">
      <h1>Play against the AI</h1>
      <p className="deck-select-sub">
        The opponent is <strong>e2</strong> (depth-2 search), tuned via {" "}
        <code>tuned_params_v6.json</code>.
      </p>

      {decksError && <div className="deck-select-error">Failed to load decks: {decksError}</div>}

      {decks && (
        <>
          <label className="deck-select-field">
            Your deck
            <select value={deckHuman} onChange={(e) => setDeckHuman(e.target.value)}>
              {decks.map((d) => (
                <option key={d.path} value={d.path}>
                  {d.label}
                </option>
              ))}
            </select>
          </label>

          <label className="deck-select-field">
            AI's deck
            <select value={deckAi} onChange={(e) => setDeckAi(e.target.value)}>
              {decks.map((d) => (
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
        </>
      )}

      {error && <div className="deck-select-error">{error}</div>}

      <button
        className="deck-select-start"
        disabled={loading || !deckHuman || !deckAi}
        onClick={() => onStart(deckHuman, deckAi, humanSeat)}
      >
        {loading ? "Starting..." : decks ? "Start game" : "Loading decks..."}
      </button>
    </div>
  );
}
