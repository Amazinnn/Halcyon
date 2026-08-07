// Real stats dashboard payload types (v1.8, mirrors Rust `DashboardPayload`).

export interface HeatmapDay {
  date: string;
  minutes: number;
}

export interface TodaySummary {
  totalSec: number;
  rounds: number;
}

export interface DashboardPayload {
  today: TodaySummary;
  heatmap30: HeatmapDay[];
  hours24: number[];
  streakDays: number;
  /** No data source yet: always null (UI shows 暂无数据). */
  distraction: null;
  idle: null;
  genres: null;
}

export function fmtDuration(sec: number): string {
  const m = Math.floor(sec / 60);
  if (m <= 0) return "0m";
  const h = Math.floor(m / 60);
  const mm = m % 60;
  if (h > 0) return `${h}h${String(mm).padStart(2, "0")}m`;
  return `${m}m`;
}