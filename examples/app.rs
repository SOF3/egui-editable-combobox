use egui_editable_combobox::{CustomOption, CustomValue, EditableComboBox, ParseDisplayValue};
use strum::IntoEnumIterator;

#[derive(Clone, PartialEq, strum::EnumIter, strum::Display, strum::EnumString)]
pub enum Continent {
    Africa,
    America,
    Antarctica,
    Eurasia,
    Oceania,
}

fn main() -> eframe::Result {
    eframe::run_native(
        "Example",
        eframe::NativeOptions::default(),
        Box::new(|_cc| {
            Ok(Box::new(App {
                value: CustomValue::Value(ParseDisplayValue(Continent::Antarctica)),
            }))
        }),
    )
}

struct App {
    value: CustomValue<ParseDisplayValue<Continent>>,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let resp = EditableComboBox::new("continent").show(
                ui,
                &mut self.value,
                Continent::iter()
                    .map(ParseDisplayValue)
                    .map(CustomOption::Value)
                    .chain([CustomOption::Custom]),
            );
            if resp.changed() {
                println!(
                    "Selected continent: {}",
                    match &self.value {
                        CustomValue::Value(v) => v.0.to_string(),
                        CustomValue::Custom(manual) => manual.clone(),
                    },
                );
            }
        });
    }
}
