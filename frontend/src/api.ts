import type { DeckInfo, GameView, NewGameResponse } from "./types";

export interface NewGameOptions {
  deckHuman: string;
  deckAi: string;
  humanSeat: number;
  seed?: number;
}

async function asJson<T>(resp: Response): Promise<T> {
  if (!resp.ok) {
    const text = await resp.text();
    throw new Error(`${resp.status}: ${text}`);
  }
  return resp.json() as Promise<T>;
}

export async function listDecks(): Promise<DeckInfo[]> {
  const resp = await fetch("/api/decks");
  return asJson<DeckInfo[]>(resp);
}

export async function createGame(opts: NewGameOptions): Promise<NewGameResponse> {
  const resp = await fetch("/api/games", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      deck_human: opts.deckHuman,
      deck_ai: opts.deckAi,
      human_seat: opts.humanSeat,
      seed: opts.seed,
    }),
  });
  return asJson<NewGameResponse>(resp);
}

export async function getGame(gameId: string): Promise<GameView> {
  const resp = await fetch(`/api/games/${gameId}`);
  return asJson<GameView>(resp);
}

export async function submitAction(gameId: string, index: number): Promise<GameView> {
  const resp = await fetch(`/api/games/${gameId}/actions`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ index }),
  });
  return asJson<GameView>(resp);
}
