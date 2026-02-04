use std::fmt::Display;
use std::str::FromStr;

use egui::IntoAtoms;

/// The selected value of an [`EditableComboBox`](crate::EditableComboBox).
pub trait Value {
    /// Converts the value to the string edited by the user.
    ///
    /// This conversion is used to populate the text editor
    /// when the user is not editing and the value is changed externally.
    fn to_editable(&self) -> String;
}

impl Value for String {
    fn to_editable(&self) -> String { self.clone() }
}

/// An option provided when displaying the list of selectable values.
///
/// `V` is the [`Value`] type this option resolves into.
/// This type is typically the same as `V` or a reference to it.
pub trait ValueOption<V> {
    /// Tests if this option matches the given text filter.
    ///
    /// `FilterState` provides context about the options filtered *before* this one.
    /// This allows implementing conditional options such as [`CustomOption`].
    fn filter_by_text(&self, text: &str, state: FilterState) -> FilterResult;

    /// Displays this option in the dropdown list.
    fn display(&self, text: &str) -> impl IntoAtoms<'_>;

    /// Converts this option into the value.
    fn into_value(self, text: &str) -> V;

    /// Tests if this option can be converted into the same value as `value`.
    fn equals_value(&self, value: &V, text: &str) -> bool;
}

/// Whether the user text fully or partially matched this option.
pub enum FilterResult {
    /// The option fully matches the user text.
    Exact,
    /// The option partially matches the user text.
    Partial,
    /// The option does not match the user text.
    None,
}

impl FilterResult {
    /// Filters `full` by allowing `input` to be a case-insensitive substring.
    pub fn from_case_insensitive_substring(
        full: impl AsRef<str>,
        input: impl AsRef<str>,
    ) -> FilterResult {
        if full.as_ref() == input.as_ref() {
            FilterResult::Exact
        } else {
            let full = full.as_ref().to_lowercase();
            let input = input.as_ref().to_lowercase();

            if full.contains(&input) { FilterResult::Partial } else { FilterResult::None }
        }
    }
}

/// State provided to [`ValueOption::filter_by_text`],
/// about the accumulated state in the current [`show`](crate::EditableComboBox::show) call.
pub struct FilterState {
    /// How many preceding options returned [`FilterResult::Partial`] or [`FilterResult::Exact`].
    pub prev_matches: usize,
    /// Whether any of the preceding options returned [`FilterResult::Exact`].
    pub had_exact:    bool,
}

impl ValueOption<String> for String {
    fn filter_by_text(&self, text: &str, _: FilterState) -> FilterResult {
        FilterResult::from_case_insensitive_substring(self, text)
    }

    fn display(&self, _text: &str) -> impl IntoAtoms<'_> { self.as_str() }

    fn into_value(self, _text: &str) -> String { self }

    fn equals_value(&self, value: &String, _text: &str) -> bool { self == value }
}

impl ValueOption<String> for &str {
    fn filter_by_text(&self, text: &str, _: FilterState) -> FilterResult {
        FilterResult::from_case_insensitive_substring(self, text)
    }

    fn display(&self, _text: &str) -> impl IntoAtoms<'_> { *self }

    fn into_value(self, _text: &str) -> String { self.to_string() }

    fn equals_value(&self, value: &String, _text: &str) -> bool { self == value }
}

/// A wrapper implementing [`Value`] and [`ValueOption`]
/// by delegating to [`FromStr`] and `Display`.
///
/// The trait bounds are particularly tailored to work with
/// [strum](https://docs.rs/strum)-deriving enums.
///
/// See [`EditableComboBox`](crate::EditableComboBox) for example usage.
pub struct ParseDisplayValue<T>(pub T);

impl<T: FromStr + Display> Value for ParseDisplayValue<T> {
    fn to_editable(&self) -> String { self.0.to_string() }
}

impl<T: FromStr + Display + PartialEq> ValueOption<ParseDisplayValue<T>> for ParseDisplayValue<T> {
    fn filter_by_text(&self, text: &str, _: FilterState) -> FilterResult {
        FilterResult::from_case_insensitive_substring(self.0.to_string(), text)
    }

    fn display(&self, _text: &str) -> impl IntoAtoms<'_> { self.0.to_string() }

    fn into_value(self, _text: &str) -> ParseDisplayValue<T> { self }

    fn equals_value(&self, value: &ParseDisplayValue<T>, _text: &str) -> bool { self.0 == value.0 }
}

/// The selected value for [`CustomOption`].
///
/// This type differs from `CustomOption` in that
/// `CustomOption` just states the existence of a "Custom" option,
/// while `CustomValue` contains the custom value entered by the user.
pub enum CustomValue<V> {
    /// User selected one of the given options.
    Value(V),
    /// User entered a custom value.
    Custom(String),
}

impl<V: Value> Value for CustomValue<V> {
    fn to_editable(&self) -> String {
        match self {
            CustomValue::Value(v) => v.to_editable(),
            CustomValue::Custom(s) => s.clone(),
        }
    }
}

/// Wraps a [`Value`] to add a "custom" option.
///
/// See [`EditableComboBox`](crate::EditableComboBox) for example usage.
pub enum CustomOption<V> {
    /// Provides an existing value option.
    Value(V),
    /// Allows entering a custom value.
    ///
    /// This option should be provided after all [`Value`](CustomOption::Value) options
    /// so that it correctly hides when a previous value was matched exactly.
    Custom,
}

enum IntoAtomsEither<A, B> {
    Left(A),
    Right(B),
}

impl<'a, A, B> IntoAtoms<'a> for IntoAtomsEither<A, B>
where
    A: IntoAtoms<'a>,
    B: IntoAtoms<'a>,
{
    fn collect(self, atoms: &mut egui::Atoms<'a>) {
        match self {
            IntoAtomsEither::Left(a) => a.collect(atoms),
            IntoAtomsEither::Right(b) => b.collect(atoms),
        }
    }
}

impl<V, Opt: ValueOption<V>> ValueOption<CustomValue<V>> for CustomOption<Opt> {
    fn filter_by_text(&self, text: &str, state: FilterState) -> FilterResult {
        match self {
            CustomOption::Value(v) => v.filter_by_text(text, state),
            CustomOption::Custom => {
                if state.had_exact {
                    FilterResult::None
                } else if state.prev_matches > 0 {
                    FilterResult::Partial
                } else {
                    FilterResult::Exact
                }
            }
        }
    }

    fn display(&self, text: &str) -> impl IntoAtoms<'_> {
        match self {
            CustomOption::Value(v) => IntoAtomsEither::Left(v.display(text)),
            CustomOption::Custom => IntoAtomsEither::Right(("Custom: ", text)),
        }
    }

    fn into_value(self, text: &str) -> CustomValue<V> {
        match self {
            CustomOption::Value(v) => CustomValue::Value(v.into_value(text)),
            CustomOption::Custom => CustomValue::Custom(text.to_string()),
        }
    }

    fn equals_value(&self, value: &CustomValue<V>, text: &str) -> bool {
        match (self, value) {
            (CustomOption::Value(this), CustomValue::Value(that)) => this.equals_value(that, text),
            (CustomOption::Custom, CustomValue::Custom(custom)) => text == custom,
            _ => false,
        }
    }
}
