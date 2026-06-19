//! Keyboard scrolling for focusable `overflow: scroll` containers (the
//! `<ScrollArea.Viewport>`). [`scroll_offset_for_key`] is the pure key→offset
//! mapping, unit-tested without a GPUI window; `render/mod.rs` attaches the GPUI
//! `on_key_down` handler that feeds the viewport's live `ScrollHandle` through it
//! and writes the clamped result back — the same handle the native scrollbar
//! tracks, so the thumb follows for free.

use gpui::Modifiers;

// Step constants — kept here, named + commented, as the single source (a future
// change could surface these as props). GPUI scroll offsets are <= 0 and the
// scrollable range is `[-max_offset, 0]`; scrolling toward the content end
// (Down / Right) makes the offset *more negative*.

/// Distance scrolled per Arrow keypress, in px (a typical line / wheel step).
const LINE_STEP: f32 = 40.0;
/// Fraction of the viewport length scrolled per Page key. < 1 so a sliver of the
/// previous page stays visible for context (browser-like).
const PAGE_FACTOR: f32 = 0.9;

/// Clamp an axis offset into the valid scroll range `[-max, 0]`.
fn clamp_offset(value: f32, max: f32) -> f32 {
    value.min(0.0).max(-max)
}

/// Map a keystroke to the new `(x, y)` scroll offset for a focusable scroll
/// container, or `None` when the key is not one this container handles (the
/// caller then leaves the offset untouched and lets the key propagate).
///
/// `has_x` / `has_y` are whether the container scrolls on each axis
/// (`overflow_x` / `overflow_y` == scroll). `cur` / `max` / `viewport` are the
/// current offset (<= 0), the max offset (`content - viewport`, >= 0), and the
/// viewport size, all per axis.
///
/// Modifier rules (C1):
/// - Ctrl / Alt / Cmd (platform) chords always pass through (`None`) so they
///   reach app-level shortcuts and are never swallowed as scrolling.
/// - Shift only participates in `Shift+Space` (= PageUp). Every other
///   `Shift+<key>` (e.g. `Shift+Arrow` for range selection) passes through.
///
/// Axis rules (C2): vertical keys (Up/Down/Page/Space) act on Y, horizontal keys
/// (Left/Right) act on X. Home/End act on Y when the container scrolls
/// vertically, otherwise on X (horizontal-only containers).
///
/// A returned offset may equal `cur` (already at a boundary, or `max == 0`); the
/// caller compares against the current offset and only consumes the key when it
/// actually moved, so boundary presses bubble to an ancestor scroller.
pub(crate) fn scroll_offset_for_key(
    key: &str,
    mods: Modifiers,
    has_x: bool,
    has_y: bool,
    cur: (f32, f32),
    max: (f32, f32),
    viewport: (f32, f32),
) -> Option<(f32, f32)> {
    // Ctrl/Alt/Cmd chords are app shortcuts, not scrolling.
    if mods.control || mods.alt || mods.platform {
        return None;
    }

    // Shift is consumed only by Shift+Space (PageUp); otherwise pass through.
    let key = if mods.shift {
        if key == "space" {
            "pageup"
        } else {
            return None;
        }
    } else {
        key
    };

    let (mut x, mut y) = cur;
    let (max_x, max_y) = max;
    let (vp_x, vp_y) = viewport;
    let _ = vp_x; // viewport width is unused today (no horizontal Page key).

    match key {
        // Vertical line steps.
        "up" if has_y => y = clamp_offset(y + LINE_STEP, max_y),
        "down" if has_y => y = clamp_offset(y - LINE_STEP, max_y),
        // Horizontal line steps.
        "left" if has_x => x = clamp_offset(x + LINE_STEP, max_x),
        "right" if has_x => x = clamp_offset(x - LINE_STEP, max_x),
        // Page steps + Space / Shift+Space (Shift+Space already rewritten above).
        "pageup" if has_y => y = clamp_offset(y + vp_y * PAGE_FACTOR, max_y),
        "pagedown" if has_y => y = clamp_offset(y - vp_y * PAGE_FACTOR, max_y),
        "space" if has_y => y = clamp_offset(y - vp_y * PAGE_FACTOR, max_y),
        // Home/End: primary axis is Y when it scrolls, else X.
        "home" => {
            if has_y {
                y = 0.0;
            } else if has_x {
                x = 0.0;
            } else {
                return None;
            }
        }
        "end" => {
            if has_y {
                y = -max_y;
            } else if has_x {
                x = -max_x;
            } else {
                return None;
            }
        }
        _ => return None,
    }

    Some((x, y))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 0.01;

    fn mods(shift: bool, control: bool, alt: bool, platform: bool) -> Modifiers {
        Modifiers {
            shift,
            control,
            alt,
            platform,
            ..Default::default()
        }
    }

    fn none() -> Modifiers {
        mods(false, false, false, false)
    }

    /// `(content 1000, viewport 100)` vertical-only container scrolled to top.
    fn vert(cur: (f32, f32)) -> Option<(f32, f32)> {
        scroll_offset_for_key(
            "down",
            none(),
            false,
            true,
            cur,
            (0.0, 900.0),
            (100.0, 100.0),
        )
    }

    fn approx(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < EPS && (a.1 - b.1).abs() < EPS
    }

    #[test]
    fn arrow_down_up_step_one_line_on_y() {
        let down = scroll_offset_for_key(
            "down",
            none(),
            false,
            true,
            (0.0, 0.0),
            (0.0, 900.0),
            (100.0, 100.0),
        )
        .unwrap();
        assert!(approx(down, (0.0, -40.0)), "{down:?}");
        let up = scroll_offset_for_key(
            "up",
            none(),
            false,
            true,
            (0.0, -100.0),
            (0.0, 900.0),
            (100.0, 100.0),
        )
        .unwrap();
        assert!(approx(up, (0.0, -60.0)), "{up:?}");
    }

    #[test]
    fn arrow_left_right_step_on_x() {
        let right = scroll_offset_for_key(
            "right",
            none(),
            true,
            false,
            (0.0, 0.0),
            (900.0, 0.0),
            (100.0, 100.0),
        )
        .unwrap();
        assert!(approx(right, (-40.0, 0.0)), "{right:?}");
        let left = scroll_offset_for_key(
            "left",
            none(),
            true,
            false,
            (-100.0, 0.0),
            (900.0, 0.0),
            (100.0, 100.0),
        )
        .unwrap();
        assert!(approx(left, (-60.0, 0.0)), "{left:?}");
    }

    #[test]
    fn page_keys_step_by_viewport_factor() {
        let pd = scroll_offset_for_key(
            "pagedown",
            none(),
            false,
            true,
            (0.0, 0.0),
            (0.0, 900.0),
            (100.0, 100.0),
        )
        .unwrap();
        assert!(approx(pd, (0.0, -90.0)), "{pd:?}"); // 100 * 0.9
        let pu = scroll_offset_for_key(
            "pageup",
            none(),
            false,
            true,
            (0.0, -200.0),
            (0.0, 900.0),
            (100.0, 100.0),
        )
        .unwrap();
        assert!(approx(pu, (0.0, -110.0)), "{pu:?}");
    }

    #[test]
    fn space_is_pagedown_shift_space_is_pageup() {
        let space = scroll_offset_for_key(
            "space",
            none(),
            false,
            true,
            (0.0, 0.0),
            (0.0, 900.0),
            (100.0, 100.0),
        )
        .unwrap();
        assert!(approx(space, (0.0, -90.0)), "{space:?}");
        let shift_space = scroll_offset_for_key(
            "space",
            mods(true, false, false, false),
            false,
            true,
            (0.0, -200.0),
            (0.0, 900.0),
            (100.0, 100.0),
        )
        .unwrap();
        assert!(approx(shift_space, (0.0, -110.0)), "{shift_space:?}");
    }

    #[test]
    fn home_end_target_primary_axis() {
        // Vertical container: Home -> top, End -> bottom (Y).
        let home = scroll_offset_for_key(
            "home",
            none(),
            false,
            true,
            (0.0, -500.0),
            (0.0, 900.0),
            (100.0, 100.0),
        )
        .unwrap();
        assert!(approx(home, (0.0, 0.0)), "{home:?}");
        let end = scroll_offset_for_key(
            "end",
            none(),
            false,
            true,
            (0.0, -100.0),
            (0.0, 900.0),
            (100.0, 100.0),
        )
        .unwrap();
        assert!(approx(end, (0.0, -900.0)), "{end:?}");
        // Horizontal-only container: Home/End act on X.
        let home_x = scroll_offset_for_key(
            "home",
            none(),
            true,
            false,
            (-500.0, 0.0),
            (900.0, 0.0),
            (100.0, 100.0),
        )
        .unwrap();
        assert!(approx(home_x, (0.0, 0.0)), "{home_x:?}");
        let end_x = scroll_offset_for_key(
            "end",
            none(),
            true,
            false,
            (-100.0, 0.0),
            (900.0, 0.0),
            (100.0, 100.0),
        )
        .unwrap();
        assert!(approx(end_x, (-900.0, 0.0)), "{end_x:?}");
    }

    #[test]
    fn clamps_at_both_ends() {
        // Down at the bottom stays at -max (no over-scroll).
        let bottom = scroll_offset_for_key(
            "down",
            none(),
            false,
            true,
            (0.0, -900.0),
            (0.0, 900.0),
            (100.0, 100.0),
        )
        .unwrap();
        assert!(approx(bottom, (0.0, -900.0)), "{bottom:?}");
        // Up at the top stays at 0.
        let top = scroll_offset_for_key(
            "up",
            none(),
            false,
            true,
            (0.0, 0.0),
            (0.0, 900.0),
            (100.0, 100.0),
        )
        .unwrap();
        assert!(approx(top, (0.0, 0.0)), "{top:?}");
        // PageUp near the top clamps to 0 rather than going positive.
        let pu = scroll_offset_for_key(
            "pageup",
            none(),
            false,
            true,
            (0.0, -50.0),
            (0.0, 900.0),
            (100.0, 100.0),
        )
        .unwrap();
        assert!(approx(pu, (0.0, 0.0)), "{pu:?}");
    }

    #[test]
    fn modifier_chords_pass_through() {
        // Ctrl / Alt / Cmd never scroll.
        assert!(vert((0.0, 0.0)).is_some()); // sanity: plain Down scrolls
        assert_eq!(
            scroll_offset_for_key(
                "down",
                mods(false, true, false, false),
                false,
                true,
                (0.0, 0.0),
                (0.0, 900.0),
                (100.0, 100.0)
            ),
            None
        );
        assert_eq!(
            scroll_offset_for_key(
                "down",
                mods(false, false, true, false),
                false,
                true,
                (0.0, 0.0),
                (0.0, 900.0),
                (100.0, 100.0)
            ),
            None
        );
        assert_eq!(
            scroll_offset_for_key(
                "down",
                mods(false, false, false, true),
                false,
                true,
                (0.0, 0.0),
                (0.0, 900.0),
                (100.0, 100.0)
            ),
            None
        );
    }

    #[test]
    fn shift_arrow_passes_through() {
        // Shift only participates in Shift+Space; Shift+Arrow passes through.
        assert_eq!(
            scroll_offset_for_key(
                "down",
                mods(true, false, false, false),
                false,
                true,
                (0.0, 0.0),
                (0.0, 900.0),
                (100.0, 100.0)
            ),
            None
        );
        assert_eq!(
            scroll_offset_for_key(
                "home",
                mods(true, false, false, false),
                false,
                true,
                (0.0, -500.0),
                (0.0, 900.0),
                (100.0, 100.0)
            ),
            None
        );
    }

    #[test]
    fn wrong_axis_keys_pass_through() {
        // Horizontal keys on a vertical-only container, and vice versa.
        assert_eq!(
            scroll_offset_for_key(
                "left",
                none(),
                false,
                true,
                (0.0, 0.0),
                (0.0, 900.0),
                (100.0, 100.0)
            ),
            None
        );
        assert_eq!(
            scroll_offset_for_key(
                "down",
                none(),
                true,
                false,
                (0.0, 0.0),
                (900.0, 0.0),
                (100.0, 100.0)
            ),
            None
        );
    }

    #[test]
    fn unknown_keys_pass_through() {
        assert_eq!(
            scroll_offset_for_key(
                "enter",
                none(),
                true,
                true,
                (0.0, 0.0),
                (900.0, 900.0),
                (100.0, 100.0)
            ),
            None
        );
        assert_eq!(
            scroll_offset_for_key(
                "a",
                none(),
                true,
                true,
                (0.0, 0.0),
                (900.0, 900.0),
                (100.0, 100.0)
            ),
            None
        );
    }

    #[test]
    fn both_axes_route_keys_independently() {
        // A container scrolling on both axes: Down -> Y, Right -> X, Home -> Y.
        let down = scroll_offset_for_key(
            "down",
            none(),
            true,
            true,
            (-10.0, -10.0),
            (900.0, 900.0),
            (100.0, 100.0),
        )
        .unwrap();
        assert!(approx(down, (-10.0, -50.0)), "{down:?}");
        let right = scroll_offset_for_key(
            "right",
            none(),
            true,
            true,
            (-10.0, -10.0),
            (900.0, 900.0),
            (100.0, 100.0),
        )
        .unwrap();
        assert!(approx(right, (-50.0, -10.0)), "{right:?}");
        let home = scroll_offset_for_key(
            "home",
            none(),
            true,
            true,
            (-10.0, -500.0),
            (900.0, 900.0),
            (100.0, 100.0),
        )
        .unwrap();
        assert!(approx(home, (-10.0, 0.0)), "{home:?}");
    }
}
