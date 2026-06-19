// TextInput entity for gluxe.
//
// Ported from crates/gpui/examples/input.rs (rev 06826ef). Key differences:
//   - `element_id` ties the entity to the gluxe tree node for prop reads and JS events.
//   - `last_sent` tracks the last value dispatched to JS to distinguish external
//     `setState` updates from our own `onChangeText` echo.
//   - Root div style is read from `StyleFields` each render so gluxe style props apply.
//   - `actions!` are declared here but registered globally in `lib.rs` (bound once).
//   - `multiline` mode: `shape_text` (soft-wrap + `\n`) instead of single-line
//     `shape_line`; the element auto-grows via a measured layout, Enter inserts a
//     newline (Cmd/Ctrl+Enter submits), and cursor/selection/hit-testing map
//     through `WrappedLine` 2D positions. The single-line path is left untouched.

use std::ops::Range;

use gpui::{
    App, AvailableSpace, Bounds, ClipboardItem, ContentMask, Context, CursorStyle,
    ElementId as GpuiElementId, ElementInputHandler, Entity, EntityInputHandler, FocusHandle,
    Focusable, GlobalElementId, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, PaintQuad, Pixels, Point, Rgba, ScrollWheelEvent, ShapedLine, SharedString,
    Style, TextRun, UTF16Selection, Window, WrappedLine, actions, div, fill, point, prelude::*, px,
    relative, rgba,
};

use crate::{
    model::{ElementId, LengthValue},
    state::{dispatch_value_event, with_tree},
    style::apply_style_props,
};

mod edit;
mod geometry;

// Pure offset/position/run helpers live in `geometry.rs`; glob-import so the
// controller and element below call them by bare name (unchanged call sites).
use edit::{extend_selection, splice_content};
use geometry::*;

// ---------------------------------------------------------------------------
// Actions (registered as keybindings in lib.rs)
// ---------------------------------------------------------------------------

actions!(
    text_input,
    [
        Backspace,
        Delete,
        Left,
        Right,
        Up,
        Down,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        SelectAll,
        Home,
        End,
        Enter,
        Submit,
        ShowCharacterPalette,
        Paste,
        Cut,
        Copy,
    ]
);

// ---------------------------------------------------------------------------
// TextInputState entity
// ---------------------------------------------------------------------------

pub(crate) struct TextInputState {
    /// The gluxe tree id for this element. Used to read props and dispatch events.
    pub(crate) element_id: ElementId,

    focus_handle: FocusHandle,

    /// Current text content (always UTF-8). May contain `\n` when `multiline`.
    content: SharedString,

    /// Placeholder shown when content is empty.
    placeholder: SharedString,

    /// Caret colour, mirrored from `style.caretColor` each render. `None` → text color.
    caret_color: Option<Rgba>,
    /// Caret width, mirrored from `style.caretWidth` each render. `None` → 1px.
    caret_width: Option<LengthValue>,
    /// Selection-highlight colour, mirrored from `style.selectionColor`. `None` → default.
    selection_color: Option<Rgba>,
    /// Placeholder text colour, mirrored from `style.placeholderColor`. `None` →
    /// built-in translucent black.
    placeholder_color: Option<Rgba>,

    /// `multiline` prop, mirrored each render. Drives Enter/paste behaviour and the
    /// `shape_text` rendering path.
    multiline: bool,
    /// Auto-grow floor / cap in rows (mirrored from `minRows`/`maxRows`).
    min_rows: Option<u32>,
    max_rows: Option<u32>,

    /// Byte-indexed selection within `content` (UTF-8 offsets).
    selected_range: Range<usize>,
    selection_reversed: bool,

    /// Active IME composition range (UTF-8 offsets).
    marked_range: Option<Range<usize>>,

    /// Cached single-line layout from the last `prepaint` (single-line mode only).
    last_layout: Option<ShapedLine>,
    /// Cached wrapped lines from the last `prepaint` (multiline mode only). Empty
    /// otherwise. Used for hit-testing, vertical navigation, and IME bounds.
    last_lines: Vec<WrappedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    /// Line height the cached layout was painted with. Event handlers and IME queries
    /// run OUTSIDE the text-style cascade, where `window.line_height()` reflects the
    /// default font rather than this input's `fontSize`; multiline geometry must use
    /// this painted value or the row mapping drifts (worse as line count grows).
    last_line_height: Pixels,

    /// Vertical scroll offset (px) for capped (`maxRows`) multiline inputs. Kept so
    /// the caret stays visible while editing; recomputed/clamped every prepaint.
    scroll_top: Pixels,
    /// Maximum scrollable offset from the last prepaint (content height − viewport).
    /// Lets the wheel handler clamp without re-shaping; 0 when the content fits.
    max_scroll: Pixels,
    /// When set, the next prepaint scrolls to keep the caret visible (the caret moved
    /// or the content changed). Cleared each prepaint so wheel scrolling isn't yanked
    /// back to the caret. Starts `true` so the initial caret is shown.
    autoscroll: bool,

    /// True while a mouse drag-select is in progress.
    is_selecting: bool,

    /// Goal column (x) preserved across consecutive Up/Down so the caret keeps its
    /// horizontal position over short lines. Cleared on any horizontal move or edit.
    goal_x: Option<Pixels>,

    /// The content string we last reported to JS via `onChangeText` (or the
    /// initial `value` prop).  Prevents the controlled-value echo from fighting
    /// the caret when `updateProps` brings back the same string we just sent.
    last_sent: String,

    /// Tracks focus state across frames so we can detect focus/blur changes in
    /// `TextElement::paint` (where we have access to the `Window`).
    was_focused: bool,
}

impl TextInputState {
    /// Called from `state::text_input_entity`. Props are re-read from the tree on every render.
    pub(crate) fn new(
        element_id: ElementId,
        initial_value: Option<String>,
        initial_placeholder: Option<String>,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        let initial_str = initial_value.unwrap_or_default();
        let content: SharedString = initial_str.clone().into();
        let cursor = content.len();

        Self {
            element_id,
            focus_handle,
            content,
            placeholder: initial_placeholder.unwrap_or_default().into(),
            caret_color: None,
            caret_width: None,
            selection_color: None,
            placeholder_color: None,
            multiline: false,
            min_rows: None,
            max_rows: None,
            selected_range: cursor..cursor,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            last_lines: Vec::new(),
            last_bounds: None,
            last_line_height: px(0.),
            scroll_top: px(0.),
            max_scroll: px(0.),
            autoscroll: true,
            is_selecting: false,
            goal_x: None,
            last_sent: initial_str,
            was_focused: false,
        }
    }

    // ---- Editing helpers --------------------------------------------------

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        // Clamp to content: when empty, the cached layout reflects the placeholder,
        // so a geometry-derived offset could otherwise point past the (empty) content.
        let offset = offset.min(self.content.len());
        self.selected_range = offset..offset;
        self.autoscroll = true;
        cx.notify();
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let offset = offset.min(self.content.len());
        (self.selected_range, self.selection_reversed) =
            extend_selection(self.selected_range.clone(), self.selection_reversed, offset);
        self.autoscroll = true;
        cx.notify();
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        let Some(bounds) = self.last_bounds.as_ref() else {
            return 0;
        };
        if self.multiline {
            // Empty content → the cached layout is the placeholder; caret stays at 0.
            if self.content.is_empty() || self.last_lines.is_empty() {
                return 0;
            }
            let local = point(
                position.x - bounds.left(),
                (position.y - bounds.top() + self.scroll_top).max(px(0.)),
            );
            return offset_for_position(&self.last_lines, local, self.last_line_height);
        }
        // Single-line.
        if self.content.is_empty() {
            return 0;
        }
        let Some(line) = self.last_layout.as_ref() else {
            return 0;
        };
        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.content.len();
        }
        line.closest_index_for_x(position.x - bounds.left())
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        previous_boundary(&self.content, offset)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        next_boundary(&self.content, offset)
    }

    /// Start of the visual row containing `offset` (multiline). Falls back to 0.
    fn visual_row_start(&self, offset: usize, line_height: Pixels) -> usize {
        let pos = position_for_offset(&self.last_lines, offset, line_height);
        offset_for_position(
            &self.last_lines,
            point(px(0.), pos.y + line_height * 0.5),
            line_height,
        )
    }

    /// End of the visual row containing `offset` (multiline). Falls back to len.
    fn visual_row_end(&self, offset: usize, line_height: Pixels) -> usize {
        let pos = position_for_offset(&self.last_lines, offset, line_height);
        offset_for_position(
            &self.last_lines,
            point(px(1.0e6), pos.y + line_height * 0.5),
            line_height,
        )
    }

    /// Called after every change to the content that should be reported to JS.
    fn notify_change(&mut self, cx: &mut Context<Self>) {
        self.goal_x = None;
        self.autoscroll = true;
        let value = self.content.to_string();
        self.last_sent = value.clone();
        cx.notify();
        dispatch_value_event(self.element_id, "change", &value);
    }

    // ---- Action handlers --------------------------------------------------

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let prev = self.previous_boundary(self.cursor_offset());
            if self.cursor_offset() == prev {
                window.play_system_bell();
                return;
            }
            self.select_to(prev, cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let next = self.next_boundary(self.cursor_offset());
            if self.cursor_offset() == next {
                window.play_system_bell();
                return;
            }
            self.select_to(next, cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        self.goal_x = None;
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx);
        }
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        self.goal_x = None;
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx);
        }
    }

    /// Move the caret to the visual row above/below, preserving the goal column.
    /// On a single-line input, Up/Down jump to the start/end of the content
    /// (matching browser `<input>` behaviour).
    fn vertical_move(&mut self, down: bool, extend: bool, cx: &mut Context<Self>) {
        if !self.multiline {
            let target = if down { self.content.len() } else { 0 };
            if extend {
                self.select_to(target, cx);
            } else {
                self.move_to(target, cx);
            }
            return;
        }
        if self.last_lines.is_empty() {
            return;
        }
        let line_height = self.last_line_height;
        let cur = self.cursor_offset();
        let pos = position_for_offset(&self.last_lines, cur, line_height);
        let goal_x = self.goal_x.unwrap_or(pos.x);
        // `pos.y` is the top of the caret's row; aim for the middle of the
        // adjacent row so the hit-test lands cleanly.
        let target_y = if down {
            pos.y + line_height * 1.5
        } else {
            pos.y - line_height * 0.5
        }
        .max(px(0.));
        let target = offset_for_position(&self.last_lines, point(goal_x, target_y), line_height);
        self.goal_x = Some(goal_x);
        if extend {
            self.select_to(target, cx);
        } else {
            self.move_to(target, cx);
        }
    }

    fn up(&mut self, _: &Up, _window: &mut Window, cx: &mut Context<Self>) {
        self.vertical_move(false, false, cx);
    }

    fn down(&mut self, _: &Down, _window: &mut Window, cx: &mut Context<Self>) {
        self.vertical_move(true, false, cx);
    }

    fn select_up(&mut self, _: &SelectUp, _window: &mut Window, cx: &mut Context<Self>) {
        self.vertical_move(false, true, cx);
    }

    fn select_down(&mut self, _: &SelectDown, _window: &mut Window, cx: &mut Context<Self>) {
        self.vertical_move(true, true, cx);
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.goal_x = None;
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.goal_x = None;
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.goal_x = None;
        self.move_to(0, cx);
        self.select_to(self.content.len(), cx);
    }

    fn home(&mut self, _: &Home, _window: &mut Window, cx: &mut Context<Self>) {
        self.goal_x = None;
        if self.multiline && !self.last_lines.is_empty() {
            let off = self.visual_row_start(self.cursor_offset(), self.last_line_height);
            self.move_to(off, cx);
        } else {
            self.move_to(0, cx);
        }
    }

    fn end(&mut self, _: &End, _window: &mut Window, cx: &mut Context<Self>) {
        self.goal_x = None;
        if self.multiline && !self.last_lines.is_empty() {
            let off = self.visual_row_end(self.cursor_offset(), self.last_line_height);
            self.move_to(off, cx);
        } else {
            self.move_to(self.content.len(), cx);
        }
    }

    fn enter(&mut self, _: &Enter, window: &mut Window, cx: &mut Context<Self>) {
        if self.multiline {
            // Enter inserts a newline; submit is bound to Cmd/Ctrl+Enter.
            self.replace_text_in_range(None, "\n", window, cx);
        } else {
            dispatch_value_event(self.element_id, "submit", self.content.as_ref());
        }
    }

    fn submit(&mut self, _: &Submit, _: &mut Window, _cx: &mut Context<Self>) {
        dispatch_value_event(self.element_id, "submit", self.content.as_ref());
    }

    fn show_character_palette(
        &mut self,
        _: &ShowCharacterPalette,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.show_character_palette();
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            // Single-line strips newlines; multiline keeps them.
            if self.multiline {
                self.replace_text_in_range(None, &text, window, cx);
            } else {
                self.replace_text_in_range(None, &text.replace('\n', " "), window, cx);
            }
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        }
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
            self.replace_text_in_range(None, "", window, cx);
        }
    }

    // ---- Mouse handlers ---------------------------------------------------

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.goal_x = None;
        self.is_selecting = true;
        let idx = self.index_for_mouse_position(event.position);
        if event.modifiers.shift {
            self.select_to(idx, cx);
        } else {
            self.move_to(idx, cx);
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _window: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_selecting {
            let idx = self.index_for_mouse_position(event.position);
            self.select_to(idx, cx);
        }
    }

    /// Scroll a capped (`maxRows`) multiline input with the wheel/trackpad. Marks the
    /// scroll as user-driven (clears `autoscroll`) so the next prepaint doesn't snap
    /// back to the caret. When not scrollable, or already at an edge, the event is left
    /// to bubble so an ancestor scroll container can take it (scroll chaining).
    fn on_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.multiline || self.max_scroll <= px(0.) {
            return;
        }
        let dy = event.delta.pixel_delta(window.line_height()).y;
        let new_scroll = (self.scroll_top - dy).min(self.max_scroll).max(px(0.));
        if new_scroll != self.scroll_top {
            self.scroll_top = new_scroll;
            self.autoscroll = false;
            cx.stop_propagation();
            cx.notify();
        }
    }

    // ---- UTF-16 conversion helpers (required by EntityInputHandler) -------

    fn offset_from_utf16(&self, offset: usize) -> usize {
        offset_from_utf16(&self.content, offset)
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        offset_to_utf16(&self.content, offset)
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }
}

// ---------------------------------------------------------------------------
// EntityInputHandler — IME / system text input
// ---------------------------------------------------------------------------

impl EntityInputHandler for TextInputState {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .or_else(|| self.marked_range.clone())
            .unwrap_or_else(|| self.selected_range.clone());

        self.content = splice_content(&self.content, &range, new_text).into();
        self.selected_range = range.start + new_text.len()..range.start + new_text.len();
        self.marked_range.take();
        self.notify_change(cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .or_else(|| self.marked_range.clone())
            .unwrap_or_else(|| self.selected_range.clone());

        self.content = splice_content(&self.content, &range, new_text).into();
        if !new_text.is_empty() {
            self.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.marked_range = None;
        }
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .map(|r| r.start + range.start..r.end + range.start)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());
        self.notify_change(cx);
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.range_from_utf16(&range_utf16);
        if self.multiline {
            if self.last_lines.is_empty() {
                return None;
            }
            let line_height = self.last_line_height;
            let s = position_for_offset(&self.last_lines, range.start, line_height);
            let e = position_for_offset(&self.last_lines, range.end, line_height);
            // Same row → span both ends; otherwise expose just the start row.
            let right = if s.y == e.y { e.x } else { s.x };
            let top = bounds.top() + s.y - self.scroll_top;
            return Some(Bounds::from_corners(
                point(bounds.left() + s.x, top),
                point(bounds.left() + right, top + line_height),
            ));
        }
        let last_layout = self.last_layout.as_ref()?;
        Some(Bounds::from_corners(
            point(
                bounds.left() + last_layout.x_for_index(range.start),
                bounds.top(),
            ),
            point(
                bounds.left() + last_layout.x_for_index(range.end),
                bounds.bottom(),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        pt: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let bounds = self.last_bounds?;
        if self.multiline {
            if self.last_lines.is_empty() {
                return None;
            }
            let local = point(
                (pt.x - bounds.left()).max(px(0.)),
                (pt.y - bounds.top() + self.scroll_top).max(px(0.)),
            );
            let utf8_index = offset_for_position(&self.last_lines, local, self.last_line_height);
            return Some(self.offset_to_utf16(utf8_index));
        }
        let line_point = bounds.localize(&pt)?;
        let last_layout = self.last_layout.as_ref()?;
        let utf8_index = last_layout.index_for_x(pt.x - line_point.x)?;
        Some(self.offset_to_utf16(utf8_index))
    }
}

// ---------------------------------------------------------------------------
// Focusable
// ---------------------------------------------------------------------------

impl Focusable for TextInputState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

impl Render for TextInputState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Read style/value/placeholder from the tree. Build the styled div inside the
        // borrow so `StyleFields` is applied by reference rather than cloned each frame.
        let (
            ext_value,
            ext_placeholder,
            tab_index,
            tab_stop,
            caret_color,
            caret_width,
            selection_color,
            placeholder_color,
            multiline,
            min_rows,
            max_rows,
            base,
        ) = with_tree(|tree| match tree.nodes.get(&self.element_id) {
            Some(e) => (
                e.props.value.clone(),
                e.props.placeholder.clone(),
                e.props.tab_index,
                // A bare <TextInput> is keyboard-reachable by default (see
                // `resolve_tab_stop`); `tabStop` overrides, `tabIndex < 0` opts out.
                e.props.resolve_tab_stop(true),
                e.props.style.caret_color,
                e.props.style.caret_width,
                e.props.style.selection_color,
                e.props.style.placeholder_color,
                e.props.multiline,
                e.props.min_rows,
                e.props.max_rows,
                apply_style_props(div(), &e.props.style),
            ),
            None => (
                None,
                None,
                None,
                true,
                None,
                None,
                None,
                None,
                false,
                None,
                None,
                div(),
            ),
        });

        // Mirror caret/selection/multiline settings onto the entity so the
        // `TextElement` (which has `&App`, not the tree) can read them.
        self.caret_color = caret_color;
        self.caret_width = caret_width;
        self.selection_color = selection_color;
        self.placeholder_color = placeholder_color;
        self.multiline = multiline;
        self.min_rows = min_rows;
        self.max_rows = max_rows;

        // Controlled-value sync: adopt external changes but ignore our own echo.
        // Skip while an IME composition is active: the external `value` lags by
        // one pump tick (updateProps is queued, not synchronous), so adopting it
        // would overwrite the in-flight `content` with a stale string, leaving
        // `marked_range` pointing into the wrong content — this underflows the
        // run-splitting in `prepaint` and crashes. Composition owns the content
        // until it commits (marked_range → None).
        if self.marked_range.is_none()
            && let Some(v) = ext_value
            && v != self.last_sent
        {
            self.content = v.clone().into();
            self.selected_range = self.content.len()..self.content.len();
            self.last_sent = v;
            self.autoscroll = true;
        }
        if let Some(p) = ext_placeholder {
            self.placeholder = p.into();
        }

        // Configure the tab order on the FocusHandle (gpui reads tab_index/tab_stop
        // from the tracked handle, not the element — see render.rs attach_focus!).
        let mut focus_handle = self.focus_handle(cx);
        if let Some(idx) = tab_index {
            focus_handle = focus_handle.tab_index(idx as isize);
        }
        focus_handle = focus_handle.tab_stop(tab_stop);

        base.key_context("TextInput")
            .track_focus(&focus_handle)
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::enter))
            .on_action(cx.listener(Self::submit))
            .on_action(cx.listener(Self::show_character_palette))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_scroll_wheel(cx.listener(Self::on_scroll_wheel))
            .child(TextElement { input: cx.entity() })
    }
}

// ---------------------------------------------------------------------------
// TextElement — custom GPUI element for cursor / selection rendering
// ---------------------------------------------------------------------------

pub(crate) struct TextElement {
    input: Entity<TextInputState>,
}

pub(crate) struct PrepaintState {
    /// Single-line shaped layout (single-line mode); `None` when multiline.
    line: Option<ShapedLine>,
    /// Wrapped lines (multiline mode); empty when single-line.
    lines: Vec<WrappedLine>,
    cursor: Option<PaintQuad>,
    /// One quad per visual row the selection covers (0 or 1 in single-line mode).
    selections: Vec<PaintQuad>,
    /// Vertical scroll offset applied to multiline painting (0 for single-line).
    scroll: Pixels,
}

impl IntoElement for TextElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<GpuiElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let multiline = self.input.read(cx).multiline;
        if !multiline {
            // Single-line: fixed one-line height, full width.
            let mut style = Style::default();
            style.size.width = relative(1.).into();
            style.size.height = window.line_height().into();
            return (window.request_layout(style, [], cx), ());
        }

        // Multiline: grow with content. The measure closure shapes the text at the
        // assigned width and returns `rows * line_height`, clamped to min/max rows.
        let min_rows = self.input.read(cx).min_rows.unwrap_or(1).max(1) as usize;
        let max_rows = self.input.read(cx).max_rows.map(|m| m.max(1) as usize);
        let line_height = window.line_height();
        let entity = self.input.clone();

        let mut style = Style::default();
        style.size.width = relative(1.).into();
        let layout_id =
            window.request_measured_layout(style, move |known, available, window, cx| {
                let wrap_width = known.width.or(match available.width {
                    AvailableSpace::Definite(w) => Some(w),
                    _ => None,
                });

                let input = entity.read(cx);
                let text: SharedString = if input.content.is_empty() {
                    input.placeholder.clone()
                } else {
                    input.content.clone()
                };
                let text_style = window.text_style();
                let font_size = text_style.font_size.to_pixels(window.rem_size());
                let run = TextRun {
                    len: text.len(),
                    font: text_style.font(),
                    color: text_style.color,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                let lines = window
                    .text_system()
                    .shape_text(text, font_size, &[run], wrap_width, None)
                    .unwrap_or_default();

                let rows: usize = lines
                    .iter()
                    .map(|l| l.wrap_boundaries().len() + 1)
                    .sum::<usize>()
                    .max(1)
                    .max(min_rows);
                let rows = max_rows.map_or(rows, |m| rows.min(m));

                // With `width: 100%` taffy takes the width from the parent, so the
                // returned width matters only during intrinsic (max-content) sizing —
                // hand back the widest unwrapped line there to avoid collapsing.
                let width = known.width.unwrap_or_else(|| {
                    let mut w = px(0.);
                    for l in &lines {
                        if l.width() > w {
                            w = l.width();
                        }
                    }
                    w
                });
                gpui::size(width, line_height * rows as f32)
            });
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let multiline = input.multiline;
        let content = input.content.clone();
        let placeholder = input.placeholder.clone();
        let selected_range = input.selected_range.clone();
        let marked_range = input.marked_range.clone();
        let cursor = input.cursor_offset();
        let caret_color = input.caret_color;
        let caret_width = input.caret_width;
        let selection_color = input.selection_color;
        let placeholder_color = input.placeholder_color;
        let prev_scroll = input.scroll_top;
        let autoscroll = input.autoscroll;
        let focused = input.focus_handle.is_focused(window);
        let style = window.text_style();

        let (display_text, text_color) = if content.is_empty() {
            // Placeholder colour: `placeholderColor` style prop, else translucent black.
            let color = placeholder_color
                .map(gpui::Hsla::from)
                .unwrap_or_else(|| gpui::hsla(0., 0., 0., 0.35));
            (placeholder, color)
        } else {
            (content, style.color)
        };

        let base_run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = build_runs(display_text.len(), base_run, marked_range.as_ref());
        let font_size = style.font_size.to_pixels(window.rem_size());

        // Caret width: px/rem only (`%`/`auto` → None), defaulting to 1px.
        let caret_w = caret_width
            .and_then(|l| l.to_absolute())
            .map(|a| a.to_pixels(window.rem_size()))
            .unwrap_or(px(1.));
        // Caret colour defaults to the text colour; selection to a translucent blue.
        let caret_fill: Rgba = caret_color.unwrap_or_else(|| style.color.into());
        let selection_fill: Rgba = selection_color.unwrap_or(rgba(0x3311ff30));

        if multiline {
            let line_height = window.line_height();
            let wrap_width = bounds.size.width.max(px(1.));
            let lines: Vec<WrappedLine> = window
                .text_system()
                .shape_text(display_text, font_size, &runs, Some(wrap_width), None)
                .unwrap_or_default()
                .into_vec();

            // Caret-follow scroll: keep the active caret row inside the (possibly
            // `maxRows`-capped) viewport. Starts from last frame's offset and is
            // clamped to the content; with no cap, `max_scroll` is 0 → no scroll.
            let total_rows = lines
                .iter()
                .map(|l| l.wrap_boundaries().len() + 1)
                .sum::<usize>()
                .max(1);
            let content_h = line_height * total_rows as f32;
            let viewport_h = bounds.size.height;
            let max_scroll = (content_h - viewport_h).max(px(0.));
            let caret = position_for_offset(&lines, cursor, line_height);
            let mut scroll = prev_scroll.min(max_scroll).max(px(0.));
            // Chase the caret only when focused and it just moved/edited (`autoscroll`);
            // a wheel-driven scroll clears the flag so the view stays where the user put
            // it, and an unfocused prefilled field shows from the top rather than the end.
            if autoscroll && focused {
                if caret.y < scroll {
                    scroll = caret.y;
                } else if caret.y + line_height > scroll + viewport_h {
                    scroll = caret.y + line_height - viewport_h;
                }
                scroll = scroll.min(max_scroll).max(px(0.));
            }
            self.input.update(cx, |input, _cx| {
                input.scroll_top = scroll;
                input.max_scroll = max_scroll;
                // Consume the request only once it's been honored (i.e. while focused),
                // so it stays pending across unfocused frames: Tab-focusing a prefilled
                // overflowing field then still reveals the caret on the first focused
                // prepaint (mouse focus already re-arms it via `move_to`).
                if focused {
                    input.autoscroll = false;
                }
            });

            let mut selections = Vec::new();
            let cursor_quad = if selected_range.is_empty() {
                Some(fill(
                    Bounds::new(
                        point(bounds.left() + caret.x, bounds.top() + caret.y - scroll),
                        gpui::size(caret_w, line_height),
                    ),
                    caret_fill,
                ))
            } else {
                push_selection_rects(
                    &lines,
                    &selected_range,
                    bounds.left(),
                    bounds.top() - scroll,
                    line_height,
                    selection_fill,
                    &mut selections,
                );
                None
            };

            return PrepaintState {
                line: None,
                lines,
                cursor: cursor_quad,
                selections,
                scroll,
            };
        }

        // Single-line.
        let line = window
            .text_system()
            .shape_line(display_text, font_size, &runs, None);

        let cursor_pos = line.x_for_index(cursor);
        let (selections, cursor_quad) = if selected_range.is_empty() {
            (
                Vec::new(),
                Some(fill(
                    Bounds::new(
                        point(bounds.left() + cursor_pos, bounds.top()),
                        gpui::size(caret_w, bounds.bottom() - bounds.top()),
                    ),
                    caret_fill,
                )),
            )
        } else {
            (
                vec![fill(
                    Bounds::from_corners(
                        point(
                            bounds.left() + line.x_for_index(selected_range.start),
                            bounds.top(),
                        ),
                        point(
                            bounds.left() + line.x_for_index(selected_range.end),
                            bounds.bottom(),
                        ),
                    ),
                    selection_fill,
                )],
                None,
            )
        };

        PrepaintState {
            line: Some(line),
            lines: Vec::new(),
            cursor: cursor_quad,
            selections,
            scroll: px(0.),
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let (focus_handle, was_focused, element_id) = {
            let input = self.input.read(cx);
            (
                input.focus_handle.clone(),
                input.was_focused,
                input.element_id,
            )
        };
        let is_focused = focus_handle.is_focused(window);

        // Detect focus/blur in paint (not via subscriptions) to avoid needing a
        // `WindowHandle` at entity construction time.
        if is_focused != was_focused {
            let content = self.input.read(cx).content.to_string();
            self.input.update(cx, |input, _cx| {
                input.was_focused = is_focused;
            });
            if is_focused {
                dispatch_value_event(element_id, "focus", &content);
            } else {
                dispatch_value_event(element_id, "blur", &content);
            }
        }

        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );

        let line_height = window.line_height();

        if let Some(line) = prepaint.line.take() {
            // Single-line.
            for selection in prepaint.selections.drain(..) {
                window.paint_quad(selection);
            }
            line.paint(
                bounds.origin,
                line_height,
                gpui::TextAlign::Left,
                None,
                window,
                cx,
            )
            .unwrap();

            if is_focused && let Some(cursor) = prepaint.cursor.take() {
                window.paint_quad(cursor);
            }

            self.input.update(cx, |input, _cx| {
                input.last_layout = Some(line);
                input.last_lines.clear();
                input.last_bounds = Some(bounds);
                input.last_line_height = line_height;
            });
        } else {
            // Multiline. Clip every quad/line to the element bounds so a
            // `maxRows`-capped (scrolled) input never paints outside its box.
            let lines = std::mem::take(&mut prepaint.lines);
            let scroll = prepaint.scroll;
            let selections = std::mem::take(&mut prepaint.selections);
            let cursor_quad = prepaint.cursor.take();
            window.with_content_mask(Some(ContentMask { bounds }), |window| {
                for selection in selections {
                    window.paint_quad(selection);
                }
                let mut y = bounds.top() - scroll;
                for line in &lines {
                    line.paint(
                        point(bounds.left(), y),
                        line_height,
                        gpui::TextAlign::Left,
                        Some(bounds),
                        window,
                        cx,
                    )
                    .unwrap();
                    y += line_height * (line.wrap_boundaries().len() + 1) as f32;
                }
                if is_focused && let Some(cursor) = cursor_quad {
                    window.paint_quad(cursor);
                }
            });

            self.input.update(cx, |input, _cx| {
                input.last_layout = None;
                input.last_lines = lines;
                input.last_bounds = Some(bounds);
                input.last_line_height = line_height;
            });
        }
    }
}
