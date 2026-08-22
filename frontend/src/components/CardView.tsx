import { useState } from "react";
import type { ActionView, Card } from "../types";
import { cardImageUrl } from "../types";
import { energyColor } from "../energyColors";
import "./CardView.css";

export function EnergyPip({ type }: { type: string }) {
  return (
    <span className="energy-pip" style={{ backgroundColor: energyColor(type) }} title={type}>
      {type[0]}
    </span>
  );
}

/** Buttons for whichever actions the caller has already matched to this card (by
 * `ActionView.hand_card_id` for hand cards, `in_play_idx` for played ones) -- so the player acts
 * directly on the card instead of hunting through a separate list. */
export function CardActions({
  actions,
  onSelect,
  disabled,
}: {
  actions: ActionView[];
  onSelect: (index: number) => void;
  disabled?: boolean;
}) {
  if (actions.length === 0) return null;
  return (
    <div className="card-actions">
      {actions.map((a) => (
        <button
          key={a.index}
          disabled={disabled}
          onClick={() => onSelect(a.index)}
          className="card-action-button"
        >
          {a.label}
        </button>
      ))}
    </div>
  );
}

/** The original text-only face -- name, type/energy color accent, and key stats. Used as a
 * fallback when the real card art (hotlinked from Limitless TCG) fails to load, so a CDN hiccup
 * or an id this project doesn't have art for degrades gracefully instead of showing nothing. */
function TextCardFace({ card, small }: { card: Card; small?: boolean }) {
  if ("Pokemon" in card) {
    const p = card.Pokemon;
    return (
      <div
        className={`card-text-face card-pokemon${small ? " card-small" : ""}`}
        style={{ borderColor: energyColor(p.energy_type) }}
      >
        <div className="card-header">
          <span className="card-name">{p.name}</span>
          <span className="card-hp">{p.hp} HP</span>
        </div>
        {!small && (
          <div className="card-body">
            {p.attacks.map((a, i) => (
              <div key={i} className="card-attack">
                <span className="attack-energy">
                  {a.energy_required.map((e, j) => (
                    <EnergyPip key={j} type={e} />
                  ))}
                </span>
                <span className="attack-title">{a.title}</span>
                <span className="attack-damage">{a.fixed_damage > 0 ? a.fixed_damage : ""}</span>
              </div>
            ))}
            {p.weakness && (
              <div className="card-meta">
                Weakness: <EnergyPip type={p.weakness} />
              </div>
            )}
          </div>
        )}
      </div>
    );
  }
  const t = card.Trainer;
  return (
    <div className={`card-text-face card-trainer${small ? " card-small" : ""}`}>
      <div className="card-header">
        <span className="card-name">{t.name}</span>
        <span className="card-type">{t.trainer_card_type}</span>
      </div>
      {!small && <div className="card-body card-effect">{t.effect}</div>}
    </div>
  );
}

/** A card face. Shows real Pokemon TCG Pocket art (hotlinked from Limitless TCG, not hosted or
 * redistributed by this project) by default, falling back to a text-only stat panel if the image
 * fails to load. */
export function CardView({
  card,
  small,
  actions,
  onSelectAction,
  actionsDisabled,
}: {
  card: Card;
  small?: boolean;
  actions?: ActionView[];
  onSelectAction?: (index: number) => void;
  actionsDisabled?: boolean;
}) {
  const [artFailed, setArtFailed] = useState(false);
  const cardActions = actions && onSelectAction && (
    <CardActions actions={actions} onSelect={onSelectAction} disabled={actionsDisabled} />
  );

  return (
    <div className={`card${small ? " card-small" : ""}`}>
      {artFailed ? (
        <TextCardFace card={card} small={small} />
      ) : (
        <img
          src={cardImageUrl(card)}
          alt={"Pokemon" in card ? card.Pokemon.name : card.Trainer.name}
          className="card-art"
          loading="lazy"
          onError={() => setArtFailed(true)}
        />
      )}
      {cardActions}
    </div>
  );
}
