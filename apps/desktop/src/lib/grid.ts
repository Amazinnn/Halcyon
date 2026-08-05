export interface GridRect { col: number; row: number; cols: number; rows: number }
export interface GridMetrics {
  cols: number;
  rows: number;
  cellW: number;
  cellH: number;
  screenW: number;
  screenH: number;
}
export function cellStyle(r: GridRect): Record<string, string> {
  return {
    gridColumn: `${r.col + 1} / span ${r.cols}`,
    gridRow: `${r.row + 1} / span ${r.rows}`,
  };
}