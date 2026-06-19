//! Pure geometry / text helpers for the `TextInput` entity.
//!
//! `shape_text` splits `content` on `\n` into one `WrappedLine` per logical line
//! (each `WrappedLine.text` excludes its trailing `\n`); each may itself soft-wrap
//! into several visual rows. These helpers map between global byte offsets in
//! `content` and 2D positions relative to the content's top-left corner, and shape
//! the selection / IME-composition runs. They are pure functions over `&str` and
//! the shaped `&[WrappedLine]` — they never touch `TextInputState` — so they live
//! apart from the controller and carry their own unit tests.

use std::ops::Range;

use gpui::{
    Bounds, PaintQuad, Pixels, Point, Rgba, TextRun, UnderlineStyle, WrappedLine, fill, point, px,
};
use unicode_segmentation::UnicodeSegmentation;

/// Byte offset of the grapheme cluster boundary immediately before `offset`.
/// Returns 0 if `offset` is at (or before) the start of `content`.
pub(super) fn previous_boundary(content: &str, offset: usize) -> usize {
    content
        .grapheme_indices(true)
        .rev()
        .find_map(|(idx, _)| (idx < offset).then_some(idx))
        .unwrap_or(0)
}

/// Byte offset of the grapheme cluster boundary immediately after `offset`.
/// Returns `content.len()` if `offset` is at (or after) the end of `content`.
pub(super) fn next_boundary(content: &str, offset: usize) -> usize {
    content
        .grapheme_indices(true)
        .find_map(|(idx, _)| (idx > offset).then_some(idx))
        .unwrap_or(content.len())
}

/// Convert a UTF-16 code-unit offset into the equivalent UTF-8 byte offset
/// within `content`.
pub(super) fn offset_from_utf16(content: &str, offset: usize) -> usize {
    let mut utf8_offset = 0;
    let mut utf16_count = 0;
    for ch in content.chars() {
        if utf16_count >= offset {
            break;
        }
        utf16_count += ch.len_utf16();
        utf8_offset += ch.len_utf8();
    }
    utf8_offset
}

/// Convert a UTF-8 byte offset into the equivalent UTF-16 code-unit offset
/// within `content`.
pub(super) fn offset_to_utf16(content: &str, offset: usize) -> usize {
    let mut utf16_offset = 0;
    let mut utf8_count = 0;
    for ch in content.chars() {
        if utf8_count >= offset {
            break;
        }
        utf8_count += ch.len_utf8();
        utf16_offset += ch.len_utf16();
    }
    utf16_offset
}

/// (logical-line index, byte offset within that line) for a global byte offset.
fn locate(lines: &[WrappedLine], offset: usize) -> (usize, usize) {
    let mut base = 0;
    for (i, line) in lines.iter().enumerate() {
        let end = base + line.len();
        if offset <= end {
            return (i, offset - base);
        }
        base = end + 1; // skip the '\n' separator
    }
    let last = lines.len().saturating_sub(1);
    (last, lines.last().map_or(0, WrappedLine::len))
}

/// Global byte offset of logical line `i`'s first character.
fn line_base(lines: &[WrappedLine], i: usize) -> usize {
    lines[..i].iter().map(|l| l.len() + 1).sum()
}

/// Number of visual rows in logical lines `[0, i)`.
fn visual_rows_before(lines: &[WrappedLine], i: usize) -> usize {
    lines[..i]
        .iter()
        .map(|l| l.wrap_boundaries().len() + 1)
        .sum()
}

/// Position of a global byte offset, relative to the content top-left.
pub(super) fn position_for_offset(
    lines: &[WrappedLine],
    offset: usize,
    line_height: Pixels,
) -> Point<Pixels> {
    if lines.is_empty() {
        return point(px(0.), px(0.));
    }
    let (li, within) = locate(lines, offset);
    let rows = visual_rows_before(lines, li);
    let local = lines[li]
        .position_for_index(within, line_height)
        .unwrap_or_else(|| point(px(0.), px(0.)));
    point(local.x, local.y + line_height * rows as f32)
}

/// Global byte offset closest to a position relative to the content top-left.
pub(super) fn offset_for_position(
    lines: &[WrappedLine],
    pos: Point<Pixels>,
    line_height: Pixels,
) -> usize {
    if lines.is_empty() {
        return 0;
    }
    let mut row_base = 0usize;
    let last = lines.len() - 1;
    for (i, line) in lines.iter().enumerate() {
        let rows = line.wrap_boundaries().len() + 1;
        let line_bottom = line_height * (row_base + rows) as f32;
        if pos.y < line_bottom || i == last {
            let local = point(pos.x, pos.y - line_height * row_base as f32);
            let within = match line.closest_index_for_position(local, line_height) {
                Ok(ix) | Err(ix) => ix,
            };
            return line_base(lines, i) + within.min(line.len());
        }
        row_base += rows;
    }
    0
}

/// Byte index at the start of each visual (soft-wrapped) sub-row of a logical line.
/// Always begins with `0`; further entries come from the wrap boundaries.
fn subrow_starts(line: &WrappedLine) -> Vec<usize> {
    let unwrapped = &line.unwrapped_layout;
    let mut starts = Vec::with_capacity(line.wrap_boundaries().len() + 1);
    starts.push(0);
    for wb in line.wrap_boundaries() {
        let idx = unwrapped
            .runs
            .get(wb.run_ix)
            .and_then(|run| run.glyphs.get(wb.glyph_ix))
            .map_or(line.len(), |glyph| glyph.index);
        starts.push(idx);
    }
    starts
}

/// Append selection quads for `range` (global byte offsets) onto `out`, one rect
/// per visual sub-row the selection touches.
pub(super) fn push_selection_rects(
    lines: &[WrappedLine],
    range: &Range<usize>,
    left: Pixels,
    top: Pixels,
    line_height: Pixels,
    color: Rgba,
    out: &mut Vec<PaintQuad>,
) {
    if range.is_empty() || lines.is_empty() {
        return;
    }
    let (start_li, _) = locate(lines, range.start);
    let (end_li, _) = locate(lines, range.end);
    for li in start_li..=end_li.min(lines.len() - 1) {
        let line = &lines[li];
        let base = line_base(lines, li);
        // Clamp the selection to this logical line's byte span.
        let a = range.start.saturating_sub(base).min(line.len());
        let b = if range.end >= base + line.len() {
            line.len()
        } else {
            range.end - base
        };
        let row_top = top + line_height * visual_rows_before(lines, li) as f32;
        let unwrapped = &line.unwrapped_layout;
        let starts = subrow_starts(line);
        for (k, &s) in starts.iter().enumerate() {
            let e = starts.get(k + 1).copied().unwrap_or(line.len());
            let os = a.max(s);
            let oe = b.min(e);
            if os >= oe {
                continue;
            }
            let row_start_x = unwrapped.x_for_index(s);
            let x0 = unwrapped.x_for_index(os) - row_start_x;
            // When the selection runs past this sub-row, fill to the row's content
            // edge (the x of the next sub-row's first glyph in the unwrapped line).
            let x1 = if b > e {
                unwrapped.x_for_index(e) - row_start_x
            } else {
                unwrapped.x_for_index(oe) - row_start_x
            };
            let y = row_top + line_height * k as f32;
            out.push(fill(
                Bounds::from_corners(point(left + x0, y), point(left + x1, y + line_height)),
                color,
            ));
        }
    }
}

/// Build the text runs for shaping, underlining the active IME composition range.
/// Lengths are clamped to `total` so a stale composition can never underflow.
pub(super) fn build_runs(
    total: usize,
    base: TextRun,
    marked: Option<&Range<usize>>,
) -> Vec<TextRun> {
    match marked {
        Some(marked_range) => {
            let start = marked_range.start.min(total);
            let end = marked_range.end.min(total);
            vec![
                TextRun {
                    len: start,
                    ..base.clone()
                },
                TextRun {
                    len: end - start,
                    underline: Some(UnderlineStyle {
                        color: Some(base.color),
                        thickness: px(1.0),
                        wavy: false,
                    }),
                    ..base.clone()
                },
                TextRun {
                    len: total - end,
                    ..base
                },
            ]
            .into_iter()
            .filter(|r| r.len > 0)
            .collect()
        }
        None => vec![base],
    }
}

#[cfg(test)]
mod tests {
    use super::{next_boundary, offset_from_utf16, offset_to_utf16, previous_boundary};

    // A man+woman+girl emoji joined by zero-width joiners: a single grapheme
    // cluster spanning many bytes / chars.
    const ZWJ_FAMILY: &str = "👨‍👩‍👧";
    // "e" followed by a combining acute accent: a single grapheme cluster.
    const COMBINING: &str = "e\u{301}";

    #[test]
    fn next_boundary_jumps_whole_zwj_cluster() {
        // From the start, the next boundary is the end of the whole cluster.
        assert_eq!(next_boundary(ZWJ_FAMILY, 0), ZWJ_FAMILY.len());
        // Probing from inside the cluster still lands at its end, never mid-cluster.
        for off in 1..ZWJ_FAMILY.len() {
            assert_eq!(next_boundary(ZWJ_FAMILY, off), ZWJ_FAMILY.len());
        }
    }

    #[test]
    fn previous_boundary_jumps_whole_zwj_cluster() {
        // From the end, the previous boundary is the start of the whole cluster.
        assert_eq!(previous_boundary(ZWJ_FAMILY, ZWJ_FAMILY.len()), 0);
        // Probing from inside the cluster still lands at its start, never mid-cluster.
        for off in 1..ZWJ_FAMILY.len() {
            assert_eq!(previous_boundary(ZWJ_FAMILY, off), 0);
        }
    }

    #[test]
    fn boundaries_jump_combining_mark_cluster() {
        // "e" + combining acute = one cluster spanning the whole string.
        assert_eq!(next_boundary(COMBINING, 0), COMBINING.len());
        assert_eq!(previous_boundary(COMBINING, COMBINING.len()), 0);
        // From inside (between the 'e' byte and the combining mark), don't split.
        assert_eq!(next_boundary(COMBINING, 1), COMBINING.len());
        assert_eq!(previous_boundary(COMBINING, 1), 0);
    }

    #[test]
    fn previous_boundary_at_start_is_zero() {
        // Backspace-at-start: nothing to delete.
        assert_eq!(previous_boundary("hello", 0), 0);
        assert_eq!(previous_boundary("", 0), 0);
        assert_eq!(previous_boundary(ZWJ_FAMILY, 0), 0);
    }

    #[test]
    fn next_boundary_at_end_is_len() {
        // Delete-at-end: nothing to delete.
        assert_eq!(next_boundary("hello", "hello".len()), "hello".len());
        assert_eq!(next_boundary("", 0), 0);
        assert_eq!(
            next_boundary(ZWJ_FAMILY, ZWJ_FAMILY.len()),
            ZWJ_FAMILY.len()
        );
    }

    #[test]
    fn boundaries_step_one_ascii_grapheme_at_a_time() {
        let s = "abc";
        assert_eq!(next_boundary(s, 0), 1);
        assert_eq!(next_boundary(s, 1), 2);
        assert_eq!(previous_boundary(s, 2), 1);
        assert_eq!(previous_boundary(s, 1), 0);
    }

    #[test]
    fn utf16_round_trip_at_char_boundaries() {
        // BMP non-ASCII (1 UTF-16 unit), astral emoji (2 UTF-16 units), ASCII.
        let s = "aé😀b"; // 'a', 'é' (U+00E9), '😀' (U+1F600), 'b'
        // Collect the UTF-8 byte offsets at each char boundary (incl. end).
        let mut byte_offsets = vec![0usize];
        for (idx, ch) in s.char_indices() {
            byte_offsets.push(idx + ch.len_utf8());
        }
        for &o in &byte_offsets {
            let utf16 = offset_to_utf16(s, o);
            assert_eq!(
                offset_from_utf16(s, utf16),
                o,
                "round-trip failed at byte offset {o}"
            );
        }
    }

    #[test]
    fn utf16_offsets_count_astral_as_two_units() {
        let s = "a😀b";
        // After 'a' (1 byte): 1 UTF-16 unit.
        assert_eq!(offset_to_utf16(s, 1), 1);
        // After 'a' + '😀' (1 + 4 bytes): 1 + 2 = 3 UTF-16 units.
        assert_eq!(offset_to_utf16(s, 1 + 4), 3);
        // The whole string: 1 + 2 + 1 = 4 UTF-16 units.
        assert_eq!(offset_to_utf16(s, s.len()), 4);
        // Inverse direction.
        assert_eq!(offset_from_utf16(s, 1), 1);
        assert_eq!(offset_from_utf16(s, 3), 1 + 4);
        assert_eq!(offset_from_utf16(s, 4), s.len());
    }
}
