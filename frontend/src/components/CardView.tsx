import type { Card } from "../types";
import { energyColor } from "../energyColors";
import "./CardView.css";

export function EnergyPip({ type }: { type: string }) {
  return (
    <span className="energy-pip" style={{ backgroundColor: energyColor(type) }} title={type}>
      {type[0]}
    </span>
  );
}

/** A compact card face -- name, type/energy color accent, and key stats. Text-only by design:
 * there's no card art anywhere in this project's data, and sourcing official Pokemon TCG Pocket
 * art raises real licensing questions worth a deliberate decision, not a default. */
export function CardView({ card, small }: { card: Card; small?: boolean }) {
  if ("Pokemon" in card) {
    const p = card.Pokemon;
    return (
      <div
        className={`card card-pokemon${small ? " card-small" : ""}`}
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
    <div className={`card card-trainer${small ? " card-small" : ""}`}>
      <div className="card-header">
        <span className="card-name">{t.name}</span>
        <span className="card-type">{t.trainer_card_type}</span>
      </div>
      {!small && <div className="card-body card-effect">{t.effect}</div>}
    </div>
  );
}
