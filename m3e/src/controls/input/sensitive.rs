//! A single-line input dedicated to short-lived sensitive responses.
//!
//! Unlike [`super::InputState`], this control never stores its value in a
//! `Rope`, edit history, search state, or the clipboard. Its owned UTF-8 buffer
//! is zeroized whenever it is superseded, cleared, cancelled, submitted, or
//! dropped.
//!
//! This guarantee is deliberately limited to the buffer owned by this control.
//! GPUI's text shaping APIs require a transient string when a prompt is shown
//! visibly, and the OS input method, renderer, compositor, or GPU may retain
//! copies outside this process-buffer guarantee. Masked rendering shapes only
//! mask characters and does not copy the plaintext into the renderer.

use std::ops::Range;

use gpui::{
    App, Bounds, Context, CursorStyle, Element, ElementId, ElementInputHandler, Entity,
    EntityInputHandler, EventEmitter, FocusHandle, Focusable, GlobalElementId,
    InteractiveElement as _, IntoElement, LayoutId, MouseButton, MouseDownEvent, PaintQuad,
    ParentElement as _, Pixels, Render, RenderOnce, ShapedLine, SharedString, Style, Styled,
    TextRun, UTF16Selection, Window, div, fill, point, prelude::FluentBuilder as _, px, relative,
    size,
};
use unicode_segmentation::UnicodeSegmentation as _;
use zeroize::Zeroize as _;

use super::{
    Backspace, Delete, Enter, Escape, InputVariant, MoveEnd, MoveHome, MoveLeft, MoveRight, Paste,
    SelectAll, input::input_style,
};
use crate::foundation::actions::{SelectLeft, SelectRight};
use crate::foundation::styled::StyleSized as _;
use crate::{ActiveTheme as _, Size, StyledExt as _};

#[cfg(test)]
use std::{cell::RefCell, rc::Rc};

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct WipeRecord {
    len: usize,
    all_zero: bool,
}

#[cfg(test)]
type WipeRecords = Rc<RefCell<Vec<WipeRecord>>>;
#[cfg(test)]
type WipeObserver = Option<WipeRecords>;
#[cfg(not(test))]
type WipeObserver = ();

/// An owned sensitive value returned by [`SensitiveInputState::take`].
///
/// Ownership can be transferred to another zeroizing wrapper with
/// [`SensitiveValue::into_bytes`] without creating an ordinary `String`.
pub struct SensitiveValue {
    bytes: Vec<u8>,
    observer: WipeObserver,
}

impl SensitiveValue {
    /// Transfer ownership of the UTF-8 bytes to another zeroizing owner.
    pub fn into_bytes(mut self) -> Vec<u8> {
        std::mem::take(&mut self.bytes)
    }

    #[cfg(test)]
    fn expose(&self) -> &str {
        // The buffer only accepts UTF-8 input.
        std::str::from_utf8(&self.bytes).unwrap()
    }
}

impl Drop for SensitiveValue {
    fn drop(&mut self) {
        wipe(&mut self.bytes, observer(&self.observer));
    }
}

#[derive(Default)]
struct SensitiveBuffer {
    bytes: Vec<u8>,
    observer: WipeObserver,
}

impl SensitiveBuffer {
    fn expose(&self) -> &str {
        // All mutations originate in `str`, so this invariant is preserved.
        unsafe { std::str::from_utf8_unchecked(&self.bytes) }
    }

    fn len(&self) -> usize {
        self.bytes.len()
    }

    fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    fn replace(&mut self, range: Range<usize>, text: &str) {
        debug_assert!(self.expose().is_char_boundary(range.start));
        debug_assert!(self.expose().is_char_boundary(range.end));

        let mut successor = Vec::with_capacity(self.len() - range.len() + text.len());
        successor.extend_from_slice(&self.bytes[..range.start]);
        successor.extend_from_slice(text.as_bytes());
        successor.extend_from_slice(&self.bytes[range.end..]);

        let mut superseded = std::mem::replace(&mut self.bytes, successor);
        wipe(&mut superseded, observer(&self.observer));
    }

    fn clear(&mut self) {
        let mut superseded = std::mem::take(&mut self.bytes);
        wipe(&mut superseded, observer(&self.observer));
    }

    fn take(&mut self) -> SensitiveValue {
        SensitiveValue {
            bytes: std::mem::take(&mut self.bytes),
            observer: clone_observer(&self.observer),
        }
    }

    #[cfg(test)]
    fn with_wipe_observer(initial: &[u8]) -> (Self, WipeRecords) {
        let records = Rc::new(RefCell::new(Vec::new()));
        (
            Self {
                bytes: initial.to_vec(),
                observer: Some(records.clone()),
            },
            records,
        )
    }
}

impl Drop for SensitiveBuffer {
    fn drop(&mut self) {
        wipe(&mut self.bytes, observer(&self.observer));
    }
}

#[cfg(not(test))]
fn observer(_: &WipeObserver) -> Option<&()> {
    None
}

#[cfg(not(test))]
fn clone_observer(_: &WipeObserver) -> WipeObserver {}

#[cfg(test)]
fn observer(observer: &WipeObserver) -> Option<&WipeRecords> {
    observer.as_ref()
}

#[cfg(test)]
fn clone_observer(observer: &WipeObserver) -> WipeObserver {
    observer.clone()
}

#[cfg(not(test))]
fn wipe(bytes: &mut Vec<u8>, _: Option<&()>) {
    bytes.zeroize();
}

#[cfg(test)]
fn wipe(bytes: &mut Vec<u8>, observer: Option<&WipeRecords>) {
    let len = bytes.len();
    bytes.zeroize();
    if let Some(observer) = observer {
        observer.borrow_mut().push(WipeRecord {
            len,
            all_zero: bytes.iter().all(|byte| *byte == 0),
        });
    }
}

/// Events emitted by a [`SensitiveInputState`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensitiveInputEvent {
    Change,
    Submit,
    Cancel,
}

/// State for a non-retaining, single-line sensitive input.
pub struct SensitiveInputState {
    focus_handle: FocusHandle,
    value: SensitiveBuffer,
    placeholder: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    masked: bool,
    disabled: bool,
}

impl SensitiveInputState {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            value: SensitiveBuffer::default(),
            placeholder: SharedString::default(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
            masked: true,
            disabled: false,
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    pub fn set_masked(&mut self, masked: bool, cx: &mut Context<Self>) {
        self.masked = masked;
        cx.notify();
    }

    pub fn set_placeholder(
        &mut self,
        placeholder: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.placeholder = placeholder.into();
        cx.notify();
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Zeroize the current value and reset all ephemeral edit state.
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.value.clear();
        self.reset_edit_state();
        cx.notify();
    }

    /// Cancel input, zeroizing the value before emitting the cancellation.
    pub fn cancel(&mut self, cx: &mut Context<Self>) {
        self.value.clear();
        self.reset_edit_state();
        cx.emit(SensitiveInputEvent::Cancel);
        cx.notify();
    }

    /// Move the value out for submission. No ordinary `String` is created.
    pub fn take(&mut self, cx: &mut Context<Self>) -> SensitiveValue {
        let value = self.value.take();
        self.reset_edit_state();
        cx.notify();
        value
    }

    fn reset_edit_state(&mut self) {
        self.selected_range = 0..0;
        self.selection_reversed = false;
        self.marked_range = None;
        self.last_layout = None;
        self.last_bounds = None;
    }

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        self.selection_reversed = false;
        cx.notify();
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset;
        } else {
            self.selected_range.end = offset;
        }
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify();
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.value
            .expose()
            .grapheme_indices(true)
            .rev()
            .find_map(|(index, _)| (index < offset).then_some(index))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.value
            .expose()
            .grapheme_indices(true)
            .find_map(|(index, _)| (index > offset).then_some(index))
            .unwrap_or(self.value.len())
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        self.value
            .expose()
            .chars()
            .take_while({
                let mut utf16 = 0;
                move |ch| {
                    if utf16 >= offset {
                        false
                    } else {
                        utf16 += ch.len_utf16();
                        true
                    }
                }
            })
            .map(char::len_utf8)
            .sum()
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        self.value.expose()[..offset]
            .chars()
            .map(char::len_utf16)
            .sum()
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range.start)..self.offset_from_utf16(range.end)
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let previous = self.previous_boundary(self.cursor_offset());
            if previous == self.cursor_offset() {
                window.play_system_bell();
                return;
            }
            self.select_to(previous, cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let next = self.next_boundary(self.cursor_offset());
            if next == self.cursor_offset() {
                window.play_system_bell();
                return;
            }
            self.select_to(next, cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn left(&mut self, _: &MoveLeft, _: &mut Window, cx: &mut Context<Self>) {
        let offset = if self.selected_range.is_empty() {
            self.previous_boundary(self.cursor_offset())
        } else {
            self.selected_range.start
        };
        self.move_to(offset, cx);
    }

    fn right(&mut self, _: &MoveRight, _: &mut Window, cx: &mut Context<Self>) {
        let offset = if self.selected_range.is_empty() {
            self.next_boundary(self.cursor_offset())
        } else {
            self.selected_range.end
        };
        self.move_to(offset, cx);
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.selected_range = 0..self.value.len();
        self.selection_reversed = false;
        cx.notify();
    }

    fn home(&mut self, _: &MoveHome, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    fn end(&mut self, _: &MoveEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.value.len(), cx);
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_text_in_range(None, &text.replace(['\n', '\r'], " "), window, cx);
        }
    }

    fn enter(&mut self, action: &Enter, _: &mut Window, cx: &mut Context<Self>) {
        if Enter::is_primary(action) {
            cx.emit(SensitiveInputEvent::Submit);
        }
    }

    fn escape(&mut self, _: &Escape, _: &mut Window, cx: &mut Context<Self>) {
        self.cancel(cx);
    }

    fn mouse_down(&mut self, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        self.move_to(self.value.len(), cx);
    }
}

impl EventEmitter<SensitiveInputEvent> for SensitiveInputState {}

impl EntityInputHandler for SensitiveInputState {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        adjusted_range.replace(self.range_to_utf16(&range));
        Some(self.value.expose()[range].to_owned())
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        let range = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or_else(|| self.marked_range.clone())
            .unwrap_or_else(|| self.selected_range.clone());
        let normalized = new_text.replace(['\n', '\r'], " ");
        self.value.replace(range.clone(), &normalized);
        let cursor = range.start + normalized.len();
        self.selected_range = cursor..cursor;
        self.marked_range = None;
        cx.emit(SensitiveInputEvent::Change);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or_else(|| self.marked_range.clone())
            .unwrap_or_else(|| self.selected_range.clone());
        self.replace_text_in_range(Some(self.range_to_utf16(&range)), new_text, window, cx);
        if !new_text.is_empty() {
            self.marked_range = Some(range.start..range.start + new_text.len());
        }
        if let Some(selected) = new_selected_range_utf16 {
            let selected = self.range_from_utf16(&selected);
            self.selected_range = range.start + selected.start..range.start + selected.end;
        }
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let line = self.last_layout.as_ref()?;
        let range = self.range_from_utf16(&range_utf16);
        Some(Bounds::from_corners(
            point(bounds.left() + line.x_for_index(range.start), bounds.top()),
            point(bounds.left() + line.x_for_index(range.end), bounds.bottom()),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        let bounds = self.last_bounds?;
        let line = self.last_layout.as_ref()?;
        let index = line.index_for_x(point.x - bounds.left())?;
        Some(self.offset_to_utf16(index))
    }
}

impl Focusable for SensitiveInputState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

struct SensitiveTextElement {
    input: Entity<SensitiveInputState>,
}

struct SensitivePrepaint {
    line: Option<ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
}

impl IntoElement for SensitiveTextElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for SensitiveTextElement {
    type RequestLayoutState = ();
    type PrepaintState = SensitivePrepaint;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> SensitivePrepaint {
        let input = self.input.read(cx);
        let selection = input.selected_range.clone();
        let cursor = input.cursor_offset();
        let style = window.text_style();
        let display_offset = |offset: usize| {
            if input.masked && !input.value.is_empty() {
                input.value.expose()[..offset].graphemes(true).count() * '•'.len_utf8()
            } else {
                offset
            }
        };
        let display: SharedString = if input.value.is_empty() {
            input.placeholder.clone()
        } else if input.masked {
            "•"
                .repeat(input.value.expose().graphemes(true).count())
                .into()
        } else {
            // Visible PAM prompts require this transient renderer copy.
            input.value.expose().to_owned().into()
        };
        let run = TextRun {
            len: display.len(),
            font: style.font(),
            color: style.color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let font_size = style.font_size.to_pixels(window.rem_size());
        let line = window
            .text_system()
            .shape_line(display, font_size, &[run], None);
        let cursor = line.x_for_index(display_offset(cursor).min(line.text.len()));
        let (selection, cursor) = if selection.is_empty() {
            (
                None,
                Some(fill(
                    Bounds::new(
                        point(bounds.left() + cursor, bounds.top()),
                        size(px(2.), bounds.size.height),
                    ),
                    style.color,
                )),
            )
        } else {
            let start = display_offset(selection.start).min(line.text.len());
            let end = display_offset(selection.end).min(line.text.len());
            (
                Some(fill(
                    Bounds::from_corners(
                        point(bounds.left() + line.x_for_index(start), bounds.top()),
                        point(bounds.left() + line.x_for_index(end), bounds.bottom()),
                    ),
                    style.color.opacity(0.2),
                )),
                None,
            )
        };
        SensitivePrepaint {
            line: Some(line),
            cursor,
            selection,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        prepaint: &mut SensitivePrepaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection);
        }
        let line = prepaint.line.take().expect("line was shaped");
        line.paint(
            bounds.origin,
            window.line_height(),
            gpui::TextAlign::Left,
            None,
            window,
            cx,
        )
        .expect("sensitive input line paints");
        if focus.is_focused(window)
            && let Some(cursor) = prepaint.cursor.take()
        {
            window.paint_quad(cursor);
        }
        self.input.update(cx, |input, _| {
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
        });
    }
}

impl Render for SensitiveInputState {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .key_context(super::CONTEXT)
            .track_focus(&self.focus_handle)
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::enter))
            .on_action(cx.listener(Self::escape))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .child(SensitiveTextElement { input: cx.entity() })
    }
}

/// Material input chrome for a [`SensitiveInputState`].
#[derive(IntoElement)]
pub struct SensitiveInput {
    state: Entity<SensitiveInputState>,
    style: gpui::StyleRefinement,
    size: Size,
    variant: InputVariant,
    invalid: bool,
    disabled: bool,
}

impl SensitiveInput {
    pub fn new(state: &Entity<SensitiveInputState>) -> Self {
        Self {
            state: state.clone(),
            style: Default::default(),
            size: Size::default(),
            variant: InputVariant::default(),
            invalid: false,
            disabled: false,
        }
    }

    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Styled for SensitiveInput {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl crate::Sizable for SensitiveInput {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for SensitiveInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.state.update(cx, |state, _| {
            state.disabled = self.disabled;
        });
        let state = self.state.read(cx);
        let focused = state.focus_handle.is_focused(window) && !self.disabled;
        let (background, _) = input_style(self.disabled, cx);
        let border = if self.invalid {
            cx.theme().error
        } else {
            cx.theme().outline_variant
        };
        div()
            .id(("sensitive-input", self.state.entity_id()))
            .flex()
            .track_focus(&state.focus_handle)
            .tab_index(0)
            .input_px(self.size)
            .input_py(self.size)
            .input_h(self.size)
            .input_text_size(self.size)
            .items_center()
            .rounded(cx.theme().radius)
            .bg(background)
            .border_1()
            .border_color(border)
            .when(focused, |this| {
                this.border_2().border_color(cx.theme().primary)
            })
            .when(self.variant == InputVariant::Filled, |this| {
                this.bg(cx.theme().surface_container_highest)
            })
            .when(self.disabled, |this| this.opacity(0.5))
            .refine_style(&self.style)
            .child(self.state)
    }
}

#[cfg(test)]
mod tests {
    use super::{SensitiveBuffer, WipeRecord};

    #[test]
    fn replacement_wipes_superseded_allocation() {
        let (mut buffer, wipes) = SensitiveBuffer::with_wipe_observer(b"old-secret");
        buffer.replace(4..10, "response");

        assert_eq!(buffer.expose(), "old-response");
        assert_eq!(
            wipes.borrow().as_slice(),
            &[WipeRecord {
                len: b"old-secret".len(),
                all_zero: true,
            }]
        );
    }

    #[test]
    fn clear_take_and_drop_are_observable_and_leave_no_recoverable_history() {
        let (mut buffer, wipes) = SensitiveBuffer::with_wipe_observer(b"first");
        buffer.clear();
        buffer.replace(0..0, "second");
        let taken = buffer.take();
        assert_eq!(taken.expose(), "second");
        drop(taken);
        drop(buffer);

        assert_eq!(
            wipes.borrow().as_slice(),
            &[
                WipeRecord {
                    len: 5,
                    all_zero: true
                },
                WipeRecord {
                    len: 0,
                    all_zero: true
                },
                WipeRecord {
                    len: 6,
                    all_zero: true
                },
                WipeRecord {
                    len: 0,
                    all_zero: true
                },
            ]
        );
    }
}
