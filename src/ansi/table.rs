use crate::ast::{TableAlignment, TableData};

use super::constants::*;
use super::inline::parse_inline_ansi;
use super::renderer::AnsiRenderer;
use super::wrap::{visible_len, wrap_ansi};

impl AnsiRenderer<'_> {
    pub(super) fn render_table(&mut self, table: &TableData) {
        let ncols = table.num_cols;
        if ncols == 0 {
            return;
        }

        // Render headers and measure column widths.
        let mut header_ansi: Vec<String> = Vec::with_capacity(ncols);
        let mut col_widths: Vec<usize> = vec![0; ncols];
        for (c, cell) in table.header.iter().enumerate().take(ncols) {
            let mut ansi = String::new();
            parse_inline_ansi(&mut ansi, cell, self.refs, self.opts, self.aopts, self.bufs);
            col_widths[c] = visible_len(&ansi);
            header_ansi.push(ansi);
        }

        // Render body cells and expand column widths as needed.
        let row_count = table.rows.len() / ncols;
        let mut rows_ansi: Vec<String> = Vec::with_capacity(table.rows.len());
        for (i, cell) in table.rows.iter().enumerate() {
            let mut ansi = String::new();
            parse_inline_ansi(&mut ansi, cell, self.refs, self.opts, self.aopts, self.bufs);
            let c = i % ncols;
            if c < ncols {
                col_widths[c] = col_widths[c].max(visible_len(&ansi));
            }
            rows_ansi.push(ansi);
        }

        let border = if self.color() { FG_BORDER } else { "" };
        let reset = if self.color() { RESET } else { "" };

        self.push_border_line('┌', '┬', '┐', &col_widths, border, reset);

        // Header row
        self.render_table_row_ansi(&header_ansi, &col_widths, &table.alignments, true);

        self.push_border_line('├', '┼', '┤', &col_widths, border, reset);

        // Data rows
        for row in 0..row_count {
            let start = row * ncols;
            let end = (start + ncols).min(rows_ansi.len());
            let row_ansi = &rows_ansi[start..end];
            self.render_table_row_ansi(row_ansi, &col_widths, &table.alignments, false);
        }

        self.push_border_line('└', '┴', '┘', &col_widths, border, reset);
        self.out.push('\n');
    }

    fn push_border_line(
        &mut self,
        left: char,
        mid: char,
        right: char,
        col_widths: &[usize],
        border: &str,
        reset: &str,
    ) {
        let ncols = col_widths.len();
        self.out.push_str(border);
        self.out.push(left);
        for (c, &w) in col_widths.iter().enumerate() {
            for _ in 0..w + 2 {
                self.out.push('─');
            }
            self.out.push(if c + 1 < ncols { mid } else { right });
        }
        self.out.push_str(reset);
        self.out.push('\n');
    }

    /// Render a table row from pre-rendered ANSI cell strings.
    fn render_table_row_ansi(
        &mut self,
        cells_ansi: &[String],
        col_widths: &[usize],
        alignments: &[TableAlignment],
        is_header: bool,
    ) {
        let border = if self.color() { FG_BORDER } else { "" };
        let reset = if self.color() { RESET } else { "" };

        let ncols = col_widths.len();
        let mut cell_lines: Vec<Vec<String>> = Vec::with_capacity(ncols);
        let mut max_lines = 1usize;
        for (c, &w) in col_widths.iter().enumerate() {
            let ansi = cells_ansi.get(c).map(|s| s.as_str()).unwrap_or("");
            let lines = if w == 0 || visible_len(ansi) <= w {
                vec![ansi.to_owned()]
            } else {
                let wrapped = wrap_ansi(ansi, w, "");
                wrapped.split('\n').map(String::from).collect()
            };
            if lines.len() > max_lines {
                max_lines = lines.len();
            }
            cell_lines.push(lines);
        }

        for line_idx in 0..max_lines {
            self.out.push_str(border);
            self.out.push('│');
            self.out.push_str(reset);

            for (c, lines) in cell_lines.iter().enumerate() {
                let w = col_widths[c];
                let align = alignments.get(c).copied().unwrap_or(TableAlignment::None);

                let cell_ansi = lines.get(line_idx).map(|s| s.as_str()).unwrap_or("");
                let vis = visible_len(cell_ansi);
                let pad = w.saturating_sub(vis);

                if is_header && self.color() {
                    self.out.push_str(BOLD);
                    self.out.push_str(FG_H1);
                }

                self.out.push(' ');
                let (lpad, rpad) = match align {
                    TableAlignment::Right => (pad, 0),
                    TableAlignment::Center => (pad / 2, pad - pad / 2),
                    _ => (0, pad),
                };
                for _ in 0..lpad {
                    self.out.push(' ');
                }
                let cell_out = cell_ansi.strip_suffix(RESET).unwrap_or(cell_ansi);
                self.out.push_str(cell_out);
                if self.color() {
                    self.out.push_str(RESET);
                }
                for _ in 0..rpad {
                    self.out.push(' ');
                }
                self.out.push(' ');
                self.out.push_str(reset);

                self.out.push_str(border);
                self.out.push('│');
                self.out.push_str(reset);
            }
            self.out.push('\n');
        }
    }
}
