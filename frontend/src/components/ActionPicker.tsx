import type { ActionView } from "../types";
import "./ActionPicker.css";

/** Actions with nowhere to live on a card -- `Board` renders the rest (place/evolve/play/attach/
 * attack/retreat/etc.) directly on their matching hand or in-play card via `ActionView.hand_card_id`
 * / `in_play_idx`. What's left here is End Turn, Draw Card, and a handful of effects that either
 * span multiple cards or can target the opponent's side (see `SimpleAction::target_hint`). */
export function ActionPicker({
  actions,
  onSelect,
  disabled,
}: {
  actions: ActionView[];
  onSelect: (index: number) => void;
  disabled: boolean;
}) {
  if (actions.length === 0) return null;
  const endTurn = actions.filter((a) => a.label === "End turn");
  const other = actions.filter((a) => a.label !== "End turn");

  return (
    <div className="action-picker">
      {other.length > 0 && (
        <div className="action-group">
          <div className="action-group-buttons">
            {other.map((a) => (
              <button
                key={a.index}
                disabled={disabled}
                onClick={() => onSelect(a.index)}
                className="action-button"
              >
                {a.label}
              </button>
            ))}
          </div>
        </div>
      )}
      {endTurn.length > 0 && (
        <div className="action-group">
          <div className="action-group-buttons">
            {endTurn.map((a) => (
              <button
                key={a.index}
                disabled={disabled}
                onClick={() => onSelect(a.index)}
                className="action-button action-button-end-turn"
              >
                {a.label}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
