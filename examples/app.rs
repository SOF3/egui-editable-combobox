use egui_editable_combobox::{EditableComboBox, ParseDisplayValue};
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
        Box::new(|_cc| Ok(Box::new(App { value: ParseDisplayValue(Continent::Antarctica) }))),
    )
}

struct App {
    value: ParseDisplayValue<Continent>,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let resp = EditableComboBox::new("continent").show(
                ui,
                &mut self.value,
                Continent::iter().map(ParseDisplayValue),
            );
            if resp.changed() {
                println!(
                    "Selected continent: {}",
                    &self.value.0,
                );
            }
        });
    }
}
