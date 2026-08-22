import { useEffect, useRef } from "react";
import type { LogEntry } from "../types";
import "./BattleLog.css";

/** Chronological list of applied actions (both players'), shown a turn at a time. Auto-scrolls
 * to the newest entry as the game progresses. */
export function BattleLog({ log, humanSeat }: { log: LogEntry[]; humanSeat: number }) {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ block: "end" });
  }, [log.length]);

  return (
    <div className="battle-log">
      {log.length === 0 ? (
        <div className="battle-log-empty">No actions yet.</div>
      ) : (
        log.map((entry, i) => {
          const isNewTurn = i === 0 || log[i - 1].turn !== entry.turn;
          return (
            <div key={i}>
              {isNewTurn && <div className="battle-log-turn-header">Turn {entry.turn}</div>}
              <div className="battle-log-entry">
                <span
                  className={
                    entry.actor === humanSeat
                      ? "battle-log-actor battle-log-you"
                      : "battle-log-actor battle-log-opponent"
                  }
                >
                  {entry.actor === humanSeat ? "You" : "AI"}
                </span>
                <span className="battle-log-label">{entry.label}</span>
              </div>
            </div>
          );
        })
      )}
      <div ref={bottomRef} />
    </div>
  );
}
