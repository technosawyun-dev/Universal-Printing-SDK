//! Shared char-grid layout for grid-constrained renderers (escpos, text).
//! html/svg lay `row` out with real proportional widths instead. Mirrors
//! `packages/core-ts/src/render/layout.ts` -- keep both in step.

use crate::types::{Align, RowColumn};

/// Largest-remainder allocation of fractional `width` units into integer chars.
fn allocate_column_widths(columns: &[RowColumn], total_chars: usize) -> Vec<usize> {
    let total_units: f64 = columns.iter().map(|c| c.width).sum();
    let raw: Vec<f64> = columns
        .iter()
        .map(|c| (c.width / total_units) * total_chars as f64)
        .collect();
    let floors: Vec<usize> = raw.iter().map(|v| v.floor() as usize).collect();
    let mut remaining = total_chars.saturating_sub(floors.iter().sum());
    let mut order: Vec<usize> = (0..raw.len()).collect();
    order.sort_by(|&a, &b| {
        let frac_a = raw[a] - floors[a] as f64;
        let frac_b = raw[b] - floors[b] as f64;
        frac_b.partial_cmp(&frac_a).unwrap()
    });
    let mut widths = floors;
    for &i in order.iter() {
        if remaining == 0 {
            break;
        }
        widths[i] += 1;
        remaining -= 1;
    }
    widths.iter().map(|&w| w.max(1)).collect()
}

fn pad_align(text: &str, width: usize, align: Align) -> String {
    let chars: Vec<char> = text.chars().collect();
    let truncated: String = if chars.len() > width {
        chars[..width].iter().collect()
    } else {
        text.to_string()
    };
    let gap = width.saturating_sub(truncated.chars().count());
    if gap == 0 {
        return truncated;
    }
    match align {
        Align::Right => format!("{}{}", " ".repeat(gap), truncated),
        Align::Center => {
            let left = gap / 2;
            format!("{}{}{}", " ".repeat(left), truncated, " ".repeat(gap - left))
        }
        Align::Left => format!("{}{}", truncated, " ".repeat(gap)),
    }
}

/// Lays out a `row` node's columns into one fixed-width line of `total_chars`.
/// Overflowing cell text is truncated rather than wrapped -- matches standard
/// receipt-table behavior (see packages/core-ts/src/render/layout.ts).
pub fn layout_row(columns: &[RowColumn], total_chars: usize) -> String {
    let widths = allocate_column_widths(columns, total_chars);
    columns
        .iter()
        .zip(widths.iter())
        .map(|(c, &w)| pad_align(&c.text, w, c.align.unwrap_or_default()))
        .collect::<Vec<_>>()
        .join("")
}

/// Greedy word-wrap; a single word longer than `width` is hard-broken.
pub fn wrap_text(value: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in value.split('\n') {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut line = String::new();
        for word in paragraph.split(' ') {
            let mut remaining: Vec<char> = word.chars().collect();
            while remaining.len() > width {
                if !line.is_empty() {
                    lines.push(std::mem::take(&mut line));
                }
                let tail = remaining.split_off(width);
                lines.push(remaining.into_iter().collect());
                remaining = tail;
            }
            let word_str: String = remaining.into_iter().collect();
            let candidate = if line.is_empty() {
                word_str.clone()
            } else {
                format!("{line} {word_str}")
            };
            if candidate.chars().count() > width {
                lines.push(std::mem::take(&mut line));
                line = word_str;
            } else {
                line = candidate;
            }
        }
        lines.push(line);
    }
    lines
}

/// Wraps `value` to `total_chars` and aligns each resulting line. Left-aligned
/// lines are returned unpadded -- nothing to align against on a standalone line.
pub fn wrap_and_align(value: &str, total_chars: usize, align: Align) -> Vec<String> {
    let lines = wrap_text(value, total_chars);
    if align == Align::Left {
        return lines;
    }
    lines.iter().map(|l| pad_align(l, total_chars, align)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col(text: &str, width: f64, align: Align) -> RowColumn {
        RowColumn { text: text.to_string(), width, align: Some(align) }
    }

    #[test]
    fn splits_columns_proportionally_to_their_width_units() {
        let line = layout_row(&[col("Latte", 3.0, Align::Left), col("x2", 1.0, Align::Center), col("6,000", 2.0, Align::Right)], 32);
        assert_eq!(line.chars().count(), 32);
        assert!(line.starts_with("Latte"));
        assert!(line.trim_end().ends_with("6,000"));
    }

    #[test]
    fn truncates_cell_text_that_overflows_its_column_width() {
        let line = layout_row(&[col("A very long item name", 1.0, Align::Left)], 8);
        assert_eq!(line, "A very l");
    }

    #[test]
    fn wraps_on_word_boundaries_without_exceeding_width() {
        let lines = wrap_text("the quick brown fox jumps", 10);
        for line in &lines {
            assert!(line.chars().count() <= 10);
        }
        assert!(lines.join(" ").contains("quick"));
    }

    #[test]
    fn hard_breaks_a_single_word_longer_than_the_width() {
        let lines = wrap_text("supercalifragilistic", 5);
        assert!(lines.iter().all(|l| l.chars().count() <= 5));
    }

    #[test]
    fn preserves_explicit_newlines_as_paragraph_breaks() {
        let lines = wrap_text("first\nsecond", 20);
        assert_eq!(lines, vec!["first".to_string(), "second".to_string()]);
    }

    #[test]
    fn wrap_and_align_pads_every_wrapped_line_to_the_target_width() {
        let lines = wrap_and_align("hi", 10, Align::Center);
        assert_eq!(lines[0].chars().count(), 10);
    }

    #[test]
    fn wrap_and_align_leaves_left_aligned_lines_unpadded() {
        let lines = wrap_and_align("hi", 10, Align::Left);
        assert_eq!(lines[0], "hi");
    }
}
