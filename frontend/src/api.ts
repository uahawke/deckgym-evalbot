import type { Card, DeckChoice, DeckInfo, DeckSummary, GameView, NewGameResponse } from "./types";

export interface NewGameOptions {
  deckHuman: DeckChoice;
  deckAi: DeckChoice;
  humanSeat: number;
  aiDepth: number;
  seed?: number;
}

function deckFields(choice: DeckChoice, pathKey: string, listKey: string): Record<string, string> {
  return "list" in choice ? { [listKey]: choice.list } : { [pathKey]: choice.path };
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

export async function listCards(): Promise<Card[]> {
  const resp = await fetch("/api/cards");
  return asJson<Card[]>(resp);
}

export async function validateDeck(list: string): Promise<DeckSummary> {
  const resp = await fetch("/api/decks/validate", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ list }),
  });
  return asJson<DeckSummary>(resp);
}

export async function createGame(opts: NewGameOptions): Promise<NewGameResponse> {
  const resp = await fetch("/api/games", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      ...deckFields(opts.deckHuman, "deck_human", "deck_human_list"),
      ...deckFields(opts.deckAi, "deck_ai", "deck_ai_list"),
      human_seat: opts.humanSeat,
      ai_depth: opts.aiDepth,
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
