//! Adds an editable combo box widget for egui.
//!
//! The main widget type is [`EditableComboBox`].
//! See its documentation for details.

#![warn(clippy::pedantic, missing_docs)]

use std::hash::Hash;

use egui::{Button, Popup, PopupAnchor, ScrollArea, TextEdit, TextStyle, TextWrapMode};

mod value;
pub use value::*;

/// A combo box that accepts text input for option filtering and custom value entry.
///
/// # Example
/// ```
/// # egui::__run_test_ui(|ui| {
/// use egui_editable_combobox::{EditableComboBox, ParseDisplayValue, Value, ValueOption};
/// use strum::IntoEnumIterator;
///
/// #[derive(Clone, PartialEq, strum::EnumIter, strum::Display, strum::EnumString)]
/// pub enum Continent {
///     Africa,
///     America,
///     Antarctica,
///     Eurasia,
///     Oceania,
/// }
///
/// let mut continent = ParseDisplayValue(Continent::Antarctica);
///
/// EditableComboBox::new("continent").show(
///     ui,
///     &mut continent,
///     Continent::iter().map(ParseDisplayValue),
/// );
/// # });
/// ```
pub struct EditableComboBox {
    id_salt: egui::Id,
}

impl EditableComboBox {
    /// Create a new `EditableComboBox` with the given ID.
    pub fn new(id_salt: impl Hash) -> Self { Self { id_salt: egui::Id::new(id_salt) } }

    /// Display the combo box as a singleline text editor in the given UI,
    /// and display a dropdown popup with the given options when focused.
    pub fn show<V, Opt>(
        self,
        ui: &mut egui::Ui,
        value: &mut V,
        options: impl IntoIterator<Item = Opt>,
    ) -> egui::Response
    where
        V: Value,
        Opt: ValueOption<V>,
    {
        let hint = value.to_editable();
        let mut text = load_text_buf(ui.ctx(), self.id_salt, value);
        let mut text_resp = TextEdit::singleline(&mut text).hint_text(&hint).show(ui).response;

        if !text_resp.has_focus() && !text_resp.lost_focus() {
            // Check that text buffer is consistent with the given value
            // when the previous frame was not focusing on the editor.

            if text != hint {
                text = hint;

                ui.ctx().request_repaint(); // repaint to apply text changes
            }
        } else if text_resp.gained_focus() {
            text.clear();
            ui.ctx().request_repaint(); // repaint to apply text changes
        }

        if text_resp.has_focus() || text_resp.lost_focus() {
            let changed = self.show_options(ui, &text_resp, value, options, &text);
            if changed {
                text_resp.mark_changed();
            }
        } else {
            self.forget_popup_state(ui.ctx());
        }

        store_text_buf(ui.ctx(), self.id_salt, text);

        text_resp
    }

    fn show_options<V, Opt>(
        &self,
        ui: &mut egui::Ui,
        text_resp: &egui::Response,
        selection: &mut V,
        options: impl IntoIterator<Item = Opt>,
        text: &str,
    ) -> bool
    where
        V: Value,
        Opt: ValueOption<V>,
    {
        let mut filtered = Vec::new();
        let mut default_cursor_pos = None;
        let mut had_exact = false;
        for (source_index, option) in options.into_iter().enumerate() {
            let equals = option.equals_value(selection, text);

            // Set default cursor position to the option matching the current value
            // when the popup is opened initially.
            if text_resp.gained_focus() && equals {
                default_cursor_pos = Some(CursorPos { source_index });
            }

            let filter_result = option
                .filter_by_text(text, FilterState { prev_matches: filtered.len(), had_exact });
            match filter_result {
                FilterResult::Partial => {
                    filtered.push(DisplayedOption { source_index, option, equals })
                }
                FilterResult::Exact => {
                    filtered.push(DisplayedOption { source_index, option, equals });
                    had_exact = true;
                }
                FilterResult::None => {}
            }
        }

        let mut cursor_pos = default_cursor_pos
            // Try to load the previous cursor position.
            .or_else(|| load_cursor_pos(ui.ctx(), self.id_salt))
            // If the previous selected value is no longer an available option,
            // reset cursor position to the first option.
            .unwrap_or(CursorPos { source_index: 0 });

        move_cursor_pos(ui.ctx(), &mut cursor_pos, &filtered);
        store_cursor_pos(ui.ctx(), self.id_salt, cursor_pos.clone());

        // Display cursor position as the smallest index greater than or equal to the current
        // cursor position, or clamp to the last one (if any) if beyond the end.
        let mut cursor_filtered_index =
            filtered.partition_point(|d| d.source_index < cursor_pos.source_index);
        if cursor_filtered_index >= filtered.len()
            && let Some(prev) = filtered.len().checked_sub(1)
        {
            cursor_filtered_index = prev;
        }

        let mut changed = false;
        Popup::new(
            Ids::Popup.id(self.id_salt),
            ui.ctx().clone(),
            PopupAnchor::ParentRect(text_resp.rect),
            ui.layer_id(),
        )
        .show(|ui| {
            ScrollArea::vertical()
                .id_salt(Ids::Scroll)
                .max_height(ui.spacing().combo_height)
                .show_rows(
                    ui,
                    ui.text_style_height(&TextStyle::Body),
                    filtered.len(),
                    |ui, range| {
                        ui.set_min_width(text_resp.rect.width());
                        ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);

                        for (filtered_index, displayed) in
                            filtered.into_iter().enumerate().take(range.end).skip(range.start)
                        {
                            let mut button = Button::selectable(
                                displayed.equals,
                                displayed.option.display(text),
                            );
                            let is_cursor = cursor_filtered_index == filtered_index;
                            if is_cursor {
                                button = button
                                    .frame_when_inactive(true)
                                    .stroke(ui.visuals().widgets.hovered.bg_stroke)
                                    .fill(ui.visuals().widgets.hovered.weak_bg_fill);
                            }
                            let select_resp = ui.add(button);
                            if select_resp.clicked()
                                || (is_cursor
                                    && ui.input(|input| input.key_pressed(egui::Key::Enter)))
                            {
                                *selection = displayed.option.into_value(text);
                                changed = true;
                            }
                        }
                    },
                );
        });

        changed
    }

    fn forget_popup_state(&self, ctx: &egui::Context) {
        ctx.memory_mut(|mem| {
            // Cursor position is no longer relevant once the popup is closed.
            // Upon reopening, the cursor position will be recalculated to match the selected value.
            mem.data.remove::<CursorPos>(Ids::CursorPos.id(self.id_salt));
        });
    }
}

fn load_text_buf<V: Value>(ctx: &egui::Context, id_salt: egui::Id, value: &V) -> String {
    ctx.memory(|mem| mem.data.get_temp::<String>(Ids::TextBuf.id(id_salt)))
        .unwrap_or_else(|| value.to_editable())
}

fn store_text_buf(ctx: &egui::Context, id_salt: egui::Id, text: String) {
    ctx.memory_mut(|mem| mem.data.insert_temp::<String>(Ids::TextBuf.id(id_salt), text));
}

fn load_cursor_pos(ctx: &egui::Context, id_salt: egui::Id) -> Option<CursorPos> {
    ctx.memory(|mem| mem.data.get_temp::<CursorPos>(Ids::CursorPos.id(id_salt)))
}

fn store_cursor_pos(ctx: &egui::Context, id_salt: egui::Id, cursor_pos: CursorPos) {
    ctx.memory_mut(|mem| mem.data.insert_temp::<CursorPos>(Ids::CursorPos.id(id_salt), cursor_pos));
}

struct DisplayedOption<Opt> {
    source_index: usize,
    option:       Opt,
    equals:       bool,
}

#[derive(Clone)]
struct CursorPos {
    source_index: usize,
}

fn move_cursor_pos<Opt>(
    ctx: &egui::Context,
    cursor_pos: &mut CursorPos,
    displayed_options: &[DisplayedOption<Opt>],
) {
    enum Motion {
        Home,
        End,
        Up,
        Down,
    }

    let Some(motion) = ctx.input(|input| {
        [
            (Motion::Up, egui::Key::ArrowUp),
            (Motion::Down, egui::Key::ArrowDown),
            (Motion::Home, egui::Key::Home),
            (Motion::End, egui::Key::End),
        ]
        .into_iter()
        .find_map(|(motion, key)| if input.key_pressed(key) { Some(motion) } else { None })
    }) else {
        return;
    };

    match motion {
        Motion::Home => {
            if let Some(first) = displayed_options.first() {
                cursor_pos.source_index = first.source_index;
            }
        }
        Motion::End => {
            if let Some(last) = displayed_options.last() {
                cursor_pos.source_index = last.source_index;
            }
        }
        Motion::Up => {
            let partition_point =
                displayed_options.partition_point(|d| d.source_index < cursor_pos.source_index);
            if let Some(new_index) = partition_point.checked_sub(1)
                && let Some(option) = displayed_options.get(new_index)
            {
                cursor_pos.source_index = option.source_index;
            } else if let Some(last) = displayed_options.last() {
                cursor_pos.source_index = last.source_index;
            }
        }
        Motion::Down => {
            let partition_point =
                displayed_options.partition_point(|d| d.source_index <= cursor_pos.source_index);
            if let Some(option) = displayed_options.get(partition_point) {
                cursor_pos.source_index = option.source_index;
            } else if let Some(first) = displayed_options.first() {
                cursor_pos.source_index = first.source_index;
            }
        }
    }
}

#[derive(Hash)]
enum Ids {
    /// Temp data key for the `TextEdit` buffer. Value has type `String`.
    TextBuf,
    /// ID salt for showing the dropdown popup.
    Popup,
    /// ID salt for the scroll area inside the popup.
    Scroll,
    /// Temp data key for storing the keyboad cursor position.
    /// Value has type `CursorPos`.
    CursorPos,
}

impl Ids {
    pub fn id(&self, salt: egui::Id) -> egui::Id { egui::Id::new((salt, self)) }
}
