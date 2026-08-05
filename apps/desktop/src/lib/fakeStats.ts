// Deterministic fake statistics (fixed seeds) so screenshots are stable.

export function seededRandom(seed: number): () => number {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return () => {
    s = (s * 16807) % 2147483647;
    return (s - 1) / 2147483646;
  };
}

export interface HeatmapDay {
  date: string;
  minutes: number;
}

const pad = (n: number) => String(n).padStart(2, "0");

export function genHeatmap30(seed = 42, days = 30): HeatmapDay[] {
  const rnd = seededRandom(seed);
  const out: HeatmapDay[] = [];
  const now = new Date();
  for (let i = days - 1; i >= 0; i--) {
    const d = new Date(now.getFullYear(), now.getMonth(), now.getDate() - i);
    const date = `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
    const minutes = Math.round(rnd() * 240);
    out.push({ date, minutes });
  }
  return out;
}

export function gen24h(seed = 7): number[] {
  const rnd = seededRandom(seed);
  return Array.from({ length: 24 }, () => Math.round(rnd() * 60));
}

export const GENRES = ["纯音乐", "白噪音", "流行", "电子", "古典"] as const;

export function genGenre(seed = 11): { genre: string; minutes: number }[] {
  const rnd = seededRandom(seed);
  return GENRES.map((genre) => ({ genre, minutes: Math.round(rnd() * 180) + 10 }));
}