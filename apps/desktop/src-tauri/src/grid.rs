//! 12x8 content-first placement grid (v1.2): floating windows are positioned
//! in logical screen cells; no overlap; text windows keep a min width.

use crate::settings::GridRect;
use serde::Serialize;

pub const GRID_COLS: usize = 12;
pub const GRID_ROWS: usize = 8;

pub const TEXT_WINDOWS: &[&str] = &["chat", "stats"];
pub const MIN_TEXT_COLS: usize = 3;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GridMetrics {
    pub cols: usize,
    pub rows: usize,
    pub cell_w: f64,
    pub cell_h: f64,
    pub screen_w: f64,
    pub screen_h: f64,
}

#[derive(Clone, Copy)]
pub struct GridManager {
    pub screen_w: f64, // logical px
    pub screen_h: f64,
}

impl GridManager {
    pub fn metrics(&self) -> GridMetrics {
        GridMetrics {
            cols: GRID_COLS,
            rows: GRID_ROWS,
            cell_w: self.screen_w / GRID_COLS as f64,
            cell_h: self.screen_h / GRID_ROWS as f64,
            screen_w: self.screen_w,
            screen_h: self.screen_h,
        }
    }

    pub fn rect_to_logical(&self, r: &GridRect) -> (f64, f64, f64, f64) {
        let m = self.metrics();
        (
            r.col as f64 * m.cell_w,
            r.row as f64 * m.cell_h,
            r.cols as f64 * m.cell_w,
            r.rows as f64 * m.cell_h,
        )
    }

    /// Validate + clamp a proposed top-left cell. Returns the final rect or
    /// Err(()) when the requested cell overlaps another window (snap back).
    pub fn place(
        &self,
        label: &str,
        current: &GridRect,
        col: usize,
        row: usize,
        occupied: &[GridRect],
    ) -> Result<GridRect, ()> {
        let mut cols = current.cols.clamp(1, GRID_COLS);
        if TEXT_WINDOWS.contains(&label) {
            cols = cols.max(MIN_TEXT_COLS).min(GRID_COLS);
        }
        let rows = current.rows.clamp(1, GRID_ROWS);

        let col = col.min(GRID_COLS - cols);
        let row = row.min(GRID_ROWS - rows);

        let candidate = GridRect { col, row, cols, rows };
        if occupied.iter().any(|o| overlap(&candidate, o)) {
            return Err(());
        }
        Ok(candidate)
    }
}

pub fn overlap(a: &GridRect, b: &GridRect) -> bool {
    a.col < b.col + b.cols && b.col < a.col + a.cols && a.row < b.row + b.rows && b.row < a.row + a.rows
}
#[cfg(test)]
mod tests {
    use super::*;

    fn gm() -> GridManager {
        GridManager { screen_w: 1536.0, screen_h: 960.0 }
    }

    fn rect(col: usize, row: usize, cols: usize, rows: usize) -> GridRect {
        GridRect { col, row, cols, rows }
    }

    #[test]
    fn place_free_cell_ok() {
        let g = gm();
        let r = g.place("chat", &rect(0, 0, 4, 4), 8, 0, &[]).unwrap();
        assert_eq!((r.col, r.row, r.cols, r.rows), (8, 0, 4, 4));
    }

    #[test]
    fn place_clamps_out_of_bounds() {
        let g = gm();
        let r = g.place("chat", &rect(0, 0, 4, 4), 99, 99, &[]).unwrap();
        assert_eq!((r.col, r.row), (GRID_COLS - 4, GRID_ROWS - 4));
    }

    #[test]
    fn place_occupied_rejected() {
        let g = gm();
        let occupied = [rect(8, 0, 4, 4)];
        assert!(g.place("stats", &rect(0, 0, 4, 3), 8, 0, &occupied).is_err());
    }

    #[test]
    fn text_window_min_width_guardrail() {
        let g = gm();
        let r = g.place("chat", &rect(0, 0, 1, 4), 0, 0, &[]).unwrap();
        assert_eq!(r.cols, MIN_TEXT_COLS);
    }

    #[test]
    fn overlap_detection() {
        assert!(overlap(&rect(0, 0, 4, 4), &rect(2, 2, 2, 2)));
        assert!(!overlap(&rect(0, 0, 2, 2), &rect(2, 0, 2, 2)));
    }

    #[test]
    fn metrics_proportional() {
        let m = gm().metrics();
        assert_eq!(m.cols, GRID_COLS);
        assert_eq!(m.rows, GRID_ROWS);
        assert!((m.cell_w - 1536.0 / 12.0).abs() < 0.001);
    }
}