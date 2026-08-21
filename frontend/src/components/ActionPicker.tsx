import type { ActionView } from "../types";
import "./ActionPicker.css";

// Rough grouping by label prefix, mirroring tui::app::action_priority_for_tui's categories.
// The label comes from SimpleAction::describe() on the Rust side -- this is a convenience
// grouping, not a structured action type, so it's necessarily a bit heuristic.
const GROUPS: { title: string; test: (label: string) => boolean }[] = [
  { title: "Place / Evolve", test: (l) => l.startsWith("Place") || l.startsWith("Evolve") },
  { title: "Play", test: (l) => l.startsWith("Play") },
  {
    title: "Attach energy / tool",
    test: (l) => l.startsWith("Attach"),
  },
  { title: "Attack", test: (l) => l.startsWith("Attack") },
  { title: "Retreat", test: (l) => l.startsWith("Retreat") },
  { title: "End turn", test: (l) => l.startsWith("End turn") },
];

export function ActionPicker({
  actions,
  onSelect,
  disabled,
}: {
  actions: ActionView[];
  onSelect: (index: number) => void;
  disabled: boolean;
}) {
  const grouped = GROUPS.map((g) => ({
    title: g.title,
    actions: actions.filter((a) => g.test(a.label)),
  })).filter((g) => g.actions.length > 0);
  const grouped_labels = new Set(grouped.flatMap((g) => g.actions.map((a) => a.index)));
  const other = actions.filter((a) => !grouped_labels.has(a.index));

  return (
    <div className="action-picker">
      {grouped.map((g) => (
        <div key={g.title} className="action-group">
          <div className="action-group-title">{g.title}</div>
          <div className="action-group-buttons">
            {g.actions.map((a) => (
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
      ))}
      {other.length > 0 && (
        <div className="action-group">
          <div className="action-group-title">Other</div>
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
    </div>
  );
}
