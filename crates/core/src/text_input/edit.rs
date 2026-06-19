// Pure text-editing operations — the content splice and the selection-extend
// surgery, lifted out of the GPUI `Context`-bound entity so they can be
// unit-tested directly. (The grapheme/UTF-16 offset math lives in `geometry.rs`;
// this module is its edit-model counterpart.) Side effects — `cx.notify()`,
// event dispatch, autoscroll — stay at the handler boundary in `mod.rs`.

use std::ops::Range;

/// Replace the UTF-8 byte `range` of `content` with `new_text`, returning the new
/// content. `range` offsets must fall on char boundaries (callers derive them
/// from the grapheme / UTF-16 helpers). This is exactly the splice both
/// `replace_text_in_range` and `replace_and_mark_text_in_range` perform.
pub(super) fn splice_content(content: &str, range: &Range<usize>, new_text: &str) -> String {
    content[..range.start].to_owned() + new_text + &content[range.end..]
}

/// Extend a selection to `offset`: move the active (non-anchored) edge, and if the
/// caret crosses the anchor, swap the ends and flip `reversed`. Returns the
/// normalised `(range, reversed)` with `start <= end`. Mirrors the in-place logic
/// in `TextInputState::select_to`.
pub(super) fn extend_selection(
    range: Range<usize>,
    reversed: bool,
    offset: usize,
) -> (Range<usize>, bool) {
    let mut start = range.start;
    let mut end = range.end;
    let mut reversed = reversed;
    if reversed {
        start = offset;
    } else {
        end = offset;
    }
    if end < start {
        reversed = !reversed;
        std::mem::swap(&mut start, &mut end);
    }
    (start..end, reversed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splice_inserts_at_empty_range() {
        assert_eq!(splice_content("ac", &(1..1), "b"), "abc");
    }

    #[test]
    fn splice_deletes_with_empty_text() {
        assert_eq!(splice_content("abc", &(1..2), ""), "ac");
    }

    #[test]
    fn splice_replaces_selection() {
        assert_eq!(
            splice_content("hello world", &(0..5), "goodbye"),
            "goodbye world"
        );
    }

    #[test]
    fn splice_at_start_and_end() {
        assert_eq!(splice_content("xyz", &(0..0), ">"), ">xyz");
        assert_eq!(splice_content("xyz", &(3..3), "<"), "xyz<");
    }

    #[test]
    fn splice_keeps_multibyte_tail_intact() {
        // "aé" = 'a' (1 byte) + 'é' (2 bytes). Replacing 'a' (0..1) must leave 'é' whole.
        assert_eq!(splice_content("aé", &(0..1), "Z"), "Zé");
    }

    #[test]
    fn extend_forward_grows_end() {
        let (range, reversed) = extend_selection(2..2, false, 5);
        assert_eq!(range, 2..5);
        assert!(!reversed);
    }

    #[test]
    fn extend_backward_past_caret_flips_reversed() {
        // Forward caret at 2; extend left to 0 → end < start → swap + flip.
        let (range, reversed) = extend_selection(2..2, false, 0);
        assert_eq!(range, 0..2);
        assert!(reversed);
    }

    #[test]
    fn extend_reversed_moves_start_edge() {
        // Reversed selection 2..5 (anchor at end=5); extend left to 1 moves start.
        let (range, reversed) = extend_selection(2..5, true, 1);
        assert_eq!(range, 1..5);
        assert!(reversed);
    }

    #[test]
    fn extend_reversed_crossing_anchor_flips_back() {
        // Reversed 2..5 (anchor at 5); extend right to 8 → start > end → swap + flip.
        let (range, reversed) = extend_selection(2..5, true, 8);
        assert_eq!(range, 5..8);
        assert!(!reversed);
    }

    #[test]
    fn extend_to_anchor_collapses_without_flipping() {
        let (range, reversed) = extend_selection(3..3, false, 3);
        assert_eq!(range, 3..3);
        assert!(!reversed);
    }
}
