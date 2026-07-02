use egui::Ui;

use crate::app::AppState;

pub fn show_toolbar(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.label("リポジトリ:");
        let response = ui.add(
            egui::TextEdit::singleline(&mut state.path_input)
                .desired_width(400.0)
                .hint_text("C:\\path\\to\\repository"),
        );
        let enter_pressed = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if ui.button("開く").clicked() || enter_pressed {
            state.needs_load = true;
        }

        if let Some(path) = &state.repo_path {
            ui.separator();
            ui.label(
                egui::RichText::new(path.to_string_lossy().as_ref())
                    .color(egui::Color32::from_gray(100))
                    .small(),
            );
        }
    });
}
