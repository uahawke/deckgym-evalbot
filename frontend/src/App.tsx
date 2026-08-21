import { useState } from "react";
import { createGame, submitAction } from "./api";
import type { GameView } from "./types";
import { DeckSelect } from "./components/DeckSelect";
import { Board } from "./components/Board";
import { ActionPicker } from "./components/ActionPicker";
import "./App.css";

function winnerText(game: GameView): string {
  if (!game.winner) return "Tie";
  if (game.winner === "Tie") return "Tie";
  return game.winner.Win === game.human_seat ? "You win!" : "AI wins.";
}

export default function App() {
  const [gameId, setGameId] = useState<string | null>(null);
  const [game, setGame] = useState<GameView | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleStart(deckHuman: string, deckAi: string, humanSeat: number) {
    setBusy(true);
    setError(null);
    try {
      const resp = await createGame({ deckHuman, deckAi, humanSeat, seed: undefined });
      setGameId(resp.game_id);
      setGame(resp);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function handleAction(index: number) {
    if (!gameId) return;
    setBusy(true);
    setError(null);
    try {
      const updated = await submitAction(gameId, index);
      setGame(updated);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  function handleRestart() {
    setGameId(null);
    setGame(null);
    setError(null);
  }

  if (!game) {
    return <DeckSelect onStart={handleStart} loading={busy} error={error} />;
  }

  return (
    <div className="app">
      <header className="app-header">
        <span>Turn {game.turn_count}</span>
        <span>{game.is_game_over ? "Game over" : game.is_human_turn ? "Your turn" : "AI's turn"}</span>
        <button className="app-restart" onClick={handleRestart}>
          New game
        </button>
      </header>

      <Board game={game} />

      {error && <div className="app-error">{error}</div>}

      {game.is_game_over ? (
        <div className="game-over-banner">{winnerText(game)}</div>
      ) : (
        <ActionPicker
          actions={game.possible_actions}
          onSelect={handleAction}
          disabled={busy || !game.is_human_turn}
        />
      )}
      {busy && !game.is_game_over && <div className="app-thinking">Thinking...</div>}
    </div>
  );
}
