const ENERGY_COLORS: Record<string, string> = {
  Grass: "#4caf50",
  Fire: "#ef5350",
  Water: "#42a5f5",
  Lightning: "#fdd835",
  Psychic: "#ab47bc",
  Fighting: "#8d6e63",
  Darkness: "#546e7a",
  Metal: "#90a4ae",
  Dragon: "#7e57c2",
  Colorless: "#bdbdbd",
};

export function energyColor(type: string | null | undefined): string {
  return (type && ENERGY_COLORS[type]) || "#bdbdbd";
}
