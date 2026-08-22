// Mirrors the JSON shapes produced by src/web/mod.rs and src/bin/server.rs on the Rust side.
// Keep in sync by hand for now -- worth generating from the Rust types later if this grows.

export type EnergyType =
  | "Grass"
  | "Fire"
  | "Water"
  | "Lightning"
  | "Psychic"
  | "Fighting"
  | "Darkness"
  | "Metal"
  | "Dragon"
  | "Colorless";

export interface Attack {
  energy_required: EnergyType[];
  title: string;
  fixed_damage: number;
  effect: string | null;
}

export interface PokemonCard {
  id: string;
  name: string;
  stage: number;
  evolves_from: string | null;
  hp: number;
  energy_type: EnergyType | null;
  ability: { title: string; effect: string } | null;
  attacks: Attack[];
  weakness: EnergyType | null;
  retreat_cost: EnergyType[];
  rarity: string;
  booster_pack: string;
}

export interface TrainerCard {
  id: string;
  trainer_card_type: "Item" | "Supporter" | "Tool" | "Fossil";
  name: string;
  effect: string;
  rarity: string;
  booster_pack: string;
}

export type Card = { Pokemon: PokemonCard } | { Trainer: TrainerCard };

export function cardName(card: Card): string {
  return "Pokemon" in card ? card.Pokemon.name : card.Trainer.name;
}

export interface PlayedCard {
  card: Card;
  damage_counters: number;
  base_hp: number;
  stadium_hp_bonus: number;
  attached_energy: EnergyType[];
  attached_tool: Card | null;
  played_this_turn: boolean;
  moved_to_active_this_turn: boolean;
  ability_used: boolean;
  poisoned: boolean;
  paralyzed: boolean;
  asleep: boolean;
  burned: boolean;
  confused: boolean;
  cards_behind: Card[];
  prevent_first_attack_damage_used: boolean;
  has_attacked_since_play: boolean;
}

export function remainingHp(pc: PlayedCard): number {
  return Math.max(0, pc.base_hp + pc.stadium_hp_bonus - pc.damage_counters);
}

export function maxHp(pc: PlayedCard): number {
  return pc.base_hp + pc.stadium_hp_bonus;
}

export interface EnergyZoneView {
  current: EnergyType | null;
  next: EnergyType | null;
}

export interface PlayerStateView {
  winner: GameOutcome | null;
  points: [number, number];
  turn_count: number;
  current_player: number;
  my_energy_zone: EnergyZoneView;
  opponent_energy_zone: EnergyZoneView;
  my_hand: Card[];
  opponent_hand_size: number;
  deck_sizes: [number, number];
  discard_piles: [Card[], Card[]];
  discard_energies: [EnergyType[], EnergyType[]];
  // index 0 of each row is the active Pokemon, 1..4 the bench. Row 0 = viewer, row 1 = opponent.
  in_play_pokemon: [
    [PlayedCard | null, PlayedCard | null, PlayedCard | null, PlayedCard | null],
    [PlayedCard | null, PlayedCard | null, PlayedCard | null, PlayedCard | null],
  ];
  active_stadium: Card | null;
  active_stadium_owner: number | null;
}

export type GameOutcome = { Win: number } | "Tie";

export interface ActionView {
  index: number;
  actor: number;
  label: string;
  hand_card_id: string | null;
  in_play_idx: number | null;
}

export interface LogEntry {
  turn: number;
  actor: number;
  label: string;
}

export interface GameView {
  turn_count: number;
  current_actor: number;
  human_seat: number;
  ai_depth: number;
  is_human_turn: boolean;
  is_game_over: boolean;
  winner: GameOutcome | null;
  points: [number, number];
  possible_actions: ActionView[];
  state: PlayerStateView;
  log: LogEntry[];
}

export interface NewGameResponse extends GameView {
  game_id: string;
}

export interface DeckInfo {
  path: string;
  label: string;
}
