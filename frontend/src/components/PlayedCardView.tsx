import type { PlayedCard } from "../types";
import { cardName, maxHp, remainingHp } from "../types";
import { EnergyPip } from "./CardView";
import { energyColor } from "../energyColors";
import "./PlayedCardView.css";

export function PlayedCardView({
  played,
  isActive,
}: {
  played: PlayedCard | null;
  isActive: boolean;
}) {
  if (!played) {
    return <div className={`slot slot-empty ${isActive ? "slot-active" : "slot-bench"}`}>—</div>;
  }
  const hp = remainingHp(played);
  const hpMax = maxHp(played);
  const hpPct = hpMax > 0 ? (hp / hpMax) * 100 : 0;
  const name = cardName(played.card);
  const energyType = "Pokemon" in played.card ? played.card.Pokemon.energy_type : null;

  const statuses: string[] = [];
  if (played.poisoned) statuses.push("Poisoned");
  if (played.paralyzed) statuses.push("Paralyzed");
  if (played.asleep) statuses.push("Asleep");
  if (played.burned) statuses.push("Burned");
  if (played.confused) statuses.push("Confused");

  return (
    <div
      className={`slot ${isActive ? "slot-active" : "slot-bench"}`}
      style={{ borderColor: energyColor(energyType) }}
    >
      <div className="slot-name">{name}</div>
      <div className="hp-bar">
        <div
          className="hp-bar-fill"
          style={{ width: `${hpPct}%`, backgroundColor: hpPct > 33 ? "#66bb6a" : "#ef5350" }}
        />
      </div>
      <div className="slot-hp-text">
        {hp} / {hpMax}
      </div>
      {played.attached_energy.length > 0 && (
        <div className="slot-energy">
          {played.attached_energy.map((e, i) => (
            <EnergyPip key={i} type={e} />
          ))}
        </div>
      )}
      {statuses.length > 0 && <div className="slot-status">{statuses.join(", ")}</div>}
      {played.attached_tool && (
        <div className="slot-tool">
          🔧{" "}
          {"Trainer" in played.attached_tool ? played.attached_tool.Trainer.name : "Tool"}
        </div>
      )}
    </div>
  );
}
