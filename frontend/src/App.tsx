import { useState } from "react";
import { createGame, submitAction } from "./api";
import type { DeckChoice, GameView } from "./types";
import { DeckSelect } from "./components/DeckSelect";
import { Board } from "./components/Board";
import { ActionPicker } from "./components/ActionPicker";
import { BattleLog } from "./components/BattleLog";
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
  const [showLog, setShowLog] = useState(false);

  async function handleStart(
    deckHuman: DeckChoice,
    deckAi: DeckChoice,
    humanSeat: number,
    aiDepth: number,
  ) {
    setBusy(true);
    setError(null);
    try {
      const resp = await createGame({ deckHuman, deckAi, humanSeat, aiDepth, seed: undefined });
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

  const actionsDisabled = busy || !game.is_human_turn;
  // Board renders place/evolve/play/attach/attack/retreat/etc. directly on their matching card
  // (see ActionView.hand_card_id/in_play_idx); ActionPicker gets only what's left over.
  const leftoverActions = game.possible_actions.filter(
    (a) => a.hand_card_id == null && a.in_play_idx == null,
  );

  return (
    <div className="app">
      <header className="app-header">
        <span>Turn {game.turn_count}</span>
        <span>{game.is_game_over ? "Game over" : game.is_human_turn ? "Your turn" : "AI's turn"}</span>
        <span>{game.ai_depth === 3 ? "Hard (e3)" : "Normal (e2)"}</span>
        <button className="app-log-toggle" onClick={() => setShowLog((s) => !s)}>
          {showLog ? "Hide log" : "Battle log"}
        </button>
        <button className="app-restart" onClick={handleRestart}>
          New game
        </button>
      </header>

      {showLog && <BattleLog log={game.log} humanSeat={game.human_seat} />}

      <Board game={game} onSelectAction={handleAction} actionsDisabled={actionsDisabled} />

      {error && <div className="app-error">{error}</div>}

      {game.is_game_over ? (
        <div className="game-over-banner">{winnerText(game)}</div>
      ) : (
        <ActionPicker actions={leftoverActions} onSelect={handleAction} disabled={actionsDisabled} />
      )}
      {busy && !game.is_game_over && <div className="app-thinking">Thinking...</div>}
    </div>
  );
}
