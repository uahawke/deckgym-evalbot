import { useState } from "react";
import type { ActionView, Card, GameView, PlayedCard } from "../types";
import { CardView, EnergyPip } from "./CardView";
import { PlayedCardView } from "./PlayedCardView";
import "./Board.css";

/** Renders one side's Pokemon row: active + 3 bench slots. `actions`/`onSelectAction` are only
 * ever passed for the viewer's own row -- `possible_actions` only exist on the human's turn, and
 * are always about the human's own board (see `SimpleAction::target_hint` on the Rust side). */
function PokemonRow({
  row,
  actions,
  onSelectAction,
  actionsDisabled,
}: {
  row: (PlayedCard | null)[];
  actions?: ActionView[];
  onSelectAction?: (index: number) => void;
  actionsDisabled?: boolean;
}) {
  const actionsFor = (idx: number) => actions?.filter((a) => a.in_play_idx === idx);
  return (
    <div className="pokemon-row">
      <PlayedCardView
        played={row[0]}
        isActive
        actions={actionsFor(0)}
        onSelectAction={onSelectAction}
        actionsDisabled={actionsDisabled}
      />
      <div className="bench-row">
        {row.slice(1).map((p, i) => (
          <PlayedCardView
            key={i}
            played={p}
            isActive={false}
            actions={actionsFor(i + 1)}
            onSelectAction={onSelectAction}
            actionsDisabled={actionsDisabled}
          />
        ))}
      </div>
    </div>
  );
}

/** A discard pile as a count, expandable in place to the actual cards -- public information in
 * TCG Pocket for both piles, on either side of the board. */
function DiscardPile({ label, cards }: { label: string; cards: Card[] }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="discard-pile">
      <button className="discard-toggle" onClick={() => setOpen((o) => !o)} disabled={cards.length === 0}>
        {label}: {cards.length}
        {cards.length > 0 && (open ? " ▲" : " ▼")}
      </button>
      {open && (
        <div className="discard-cards">
          {cards.map((c, i) => (
            <CardView key={i} card={c} small />
          ))}
        </div>
      )}
    </div>
  );
}

export function Board({
  game,
  onSelectAction,
  actionsDisabled,
}: {
  game: GameView;
  onSelectAction?: (index: number) => void;
  actionsDisabled?: boolean;
}) {
  const { state, human_seat, possible_actions } = game;
  // points/current_actor/winner are indexed by raw seat number; everything inside `state` is
  // already reindexed [mine, opponent's] by the backend. See types.ts.
  const myPoints = game.points[human_seat];
  const opponentPoints = game.points[(human_seat + 1) % 2];

  const [myRow, opponentRow] = state.in_play_pokemon;
  const [myDiscard, opponentDiscard] = state.discard_piles;
  const [myDeckSize, opponentDeckSize] = state.deck_sizes;

  const handActions = (cardId: string) =>
    possible_actions.filter((a) => a.hand_card_id === cardId);
  // Actions with neither a hand card nor an in-play slot to live on (EndTurn, DrawCard, and a
  // handful of multi-card/opponent-side effects) still need somewhere to appear -- the caller
  // (App.tsx) renders these via ActionPicker alongside the board.

  return (
    <div className="board">
      <section className="side side-opponent">
        <div className="side-header">
          <span className="side-label">Opponent</span>
          <span className="side-points">Points: {opponentPoints}</span>
          <span className="side-stat">Deck: {opponentDeckSize}</span>
          <span className="side-stat">Hand: {state.opponent_hand_size}</span>
          {state.opponent_energy_zone.current && (
            <span className="side-stat">
              Energy: <EnergyPip type={state.opponent_energy_zone.current} />
            </span>
          )}
          {state.opponent_energy_zone.next && (
            <span className="side-stat">
              Next: <EnergyPip type={state.opponent_energy_zone.next} />
            </span>
          )}
        </div>
        <PokemonRow row={opponentRow} />
        <div className="opponent-hand">
          {Array.from({ length: state.opponent_hand_size }).map((_, i) => (
            <div key={i} className="card-back" />
          ))}
        </div>
        <DiscardPile label="Discard" cards={opponentDiscard} />
      </section>

      {state.active_stadium && (
        <div className="stadium">
          Stadium: <CardView card={state.active_stadium} small />
        </div>
      )}

      <section className="side side-mine">
        <PokemonRow
          row={myRow}
          actions={possible_actions}
          onSelectAction={onSelectAction}
          actionsDisabled={actionsDisabled}
        />
        <div className="side-header">
          <span className="side-label">You</span>
          <span className="side-points">Points: {myPoints}</span>
          <span className="side-stat">Deck: {myDeckSize}</span>
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
        <DiscardPile label="Discard" cards={myDiscard} />
        <div className="my-hand">
          {state.my_hand.map((card, i) => {
            const id = "Pokemon" in card ? card.Pokemon.id : card.Trainer.id;
            return (
              <CardView
                key={i}
                card={card}
                actions={handActions(id)}
                onSelectAction={onSelectAction}
                actionsDisabled={actionsDisabled}
              />
            );
          })}
        </div>
      </section>
    </div>
  );
}
