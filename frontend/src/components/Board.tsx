import type { GameView, PlayedCard } from "../types";
import { CardView, EnergyPip } from "./CardView";
import { PlayedCardView } from "./PlayedCardView";
import "./Board.css";

/** Renders one side's Pokemon row: active + 3 bench slots. */
function PokemonRow({ row }: { row: (PlayedCard | null)[] }) {
  return (
    <div className="pokemon-row">
      <PlayedCardView played={row[0]} isActive />
      <div className="bench-row">
        {row.slice(1).map((p, i) => (
          <PlayedCardView key={i} played={p} isActive={false} />
        ))}
      </div>
    </div>
  );
}

export function Board({ game }: { game: GameView }) {
  const { state, human_seat } = game;
  // points/current_actor/winner are indexed by raw seat number; everything inside `state` is
  // already reindexed [mine, opponent's] by the backend. See types.ts.
  const myPoints = game.points[human_seat];
  const opponentPoints = game.points[(human_seat + 1) % 2];

  const [myRow, opponentRow] = state.in_play_pokemon;
  const [myDiscard, opponentDiscard] = state.discard_piles;
  const [myDeckSize, opponentDeckSize] = state.deck_sizes;

  return (
    <div className="board">
      <section className="side side-opponent">
        <div className="side-header">
          <span className="side-label">Opponent</span>
          <span className="side-points">Points: {opponentPoints}</span>
          <span className="side-stat">Deck: {opponentDeckSize}</span>
          <span className="side-stat">Hand: {state.opponent_hand_size}</span>
          <span className="side-stat">Discard: {opponentDiscard.length}</span>
          {state.opponent_energy_current && (
            <span className="side-stat">
              Energy: <EnergyPip type={state.opponent_energy_current} />
            </span>
          )}
        </div>
        <PokemonRow row={opponentRow} />
        <div className="opponent-hand">
          {Array.from({ length: state.opponent_hand_size }).map((_, i) => (
            <div key={i} className="card-back" />
          ))}
        </div>
      </section>

      {state.active_stadium && (
        <div className="stadium">
          Stadium: <CardView card={state.active_stadium} small />
        </div>
      )}

      <section className="side side-mine">
        <PokemonRow row={myRow} />
        <div className="side-header">
          <span className="side-label">You</span>
          <span className="side-points">Points: {myPoints}</span>
          <span className="side-stat">Deck: {myDeckSize}</span>
          <span className="side-stat">Discard: {myDiscard.length}</span>
          {state.my_energy_zone.current && (
            <span className="side-stat">
              Energy: <EnergyPip type={state.my_energy_zone.current} />
            </span>
          )}
          {state.my_energy_zone.next && (
            <span className="side-stat">
              Next: <EnergyPip type={state.my_energy_zone.next} />
            </span>
          )}
        </div>
        <div className="my-hand">
          {state.my_hand.map((card, i) => (
            <CardView key={i} card={card} />
          ))}
        </div>
      </section>
    </div>
  );
}
