use egui::{Color32, RichText, Ui};

use crate::{
    app::AppState,
    git::{DiffLineKind, FileStatus},
};

const COLOR_ADDED_BG: Color32 = Color32::from_rgb(221, 244, 220);
const COLOR_DELETED_BG: Color32 = Color32::from_rgb(255, 220, 220);
const COLOR_CONTEXT_BG: Color32 = Color32::from_rgb(250, 250, 250);
const COLOR_ADDED_TEXT: Color32 = Color32::from_rgb(0, 100, 0);
const COLOR_DELETED_TEXT: Color32 = Color32::from_rgb(150, 0, 0);
const COLOR_HUNK_HEADER_BG: Color32 = Color32::from_rgb(219, 234, 254);
const COLOR_HUNK_HEADER_TEXT: Color32 = Color32::from_rgb(0, 70, 140);
const COLOR_FILE_SELECTED: Color32 = Color32::from_rgb(232, 240, 254);
const COLOR_META: Color32 = Color32::from_rgb(130, 130, 130);
const COLOR_ADDED_BADGE: Color32 = Color32::from_rgb(40, 167, 69);
const COLOR_DELETED_BADGE: Color32 = Color32::from_rgb(209, 36, 47);
const COLOR_MODIFIED_BADGE: Color32 = Color32::from_rgb(0, 117, 202);
const COLOR_RENAMED_BADGE: Color32 = Color32::from_rgb(108, 117, 125);

pub fn show_diff_panel(ui: &mut Ui, state: &mut AppState) {
    if state.selected_commit.is_none() {
        ui.centered_and_justified(|ui| {
            ui.label(
                RichText::new("コミットを選択してください")
                    .color(Color32::from_gray(150))
                    .size(16.0),
            );
        });
        return;
    }

    let available_height = ui.available_height();
    let file_list_height = (available_height * 0.28).clamp(80.0, 240.0);

    ui.allocate_ui(egui::vec2(ui.available_width(), file_list_height), |ui| {
        show_file_list(ui, state);
    });

    ui.separator();

    show_diff_view(ui, state);
}

fn show_file_list(ui: &mut Ui, state: &mut AppState) {
    if state.diff_files.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(
                RichText::new("変更ファイルなし")
                    .color(Color32::from_gray(160))
                    .size(13.0),
            );
        });
        return;
    }

    egui::ScrollArea::vertical()
        .id_salt("file_list_scroll")

        .show(ui, |ui| {
            let mut clicked_file: Option<usize> = None;

            for (idx, file) in state.diff_files.iter().enumerate() {
                let is_selected = state.selected_file == Some(idx);
                let bg = if is_selected {
                    COLOR_FILE_SELECTED
                } else {
                    Color32::TRANSPARENT
                };

                let response = egui::Frame::new()
                    .fill(bg)
                    .inner_margin(egui::Margin::symmetric(6, 3))
                    .show(ui, |ui| {

                        ui.horizontal(|ui| {
                            let (badge_text, badge_color) = match &file.status {
                                FileStatus::Added => ("A", COLOR_ADDED_BADGE),
                                FileStatus::Deleted => ("D", COLOR_DELETED_BADGE),
                                FileStatus::Modified => ("M", COLOR_MODIFIED_BADGE),
                                FileStatus::Renamed { .. } => ("R", COLOR_RENAMED_BADGE),
                            };
                            egui::Frame::new()
                                .fill(badge_color)
                                .corner_radius(3)
                                .inner_margin(egui::Margin::symmetric(4, 1))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(badge_text)
                                            .color(Color32::WHITE)
                                            .monospace()
                                            .size(11.0),
                                    );
                                });

                            ui.add(
                                egui::Label::new(
                                    RichText::new(&file.path).monospace().size(12.0),
                                )
                                .truncate(),
                            );

                            if file.is_binary {
                                ui.label(
                                    RichText::new("(binary)").color(COLOR_META).size(11.0),
                                );
                            }

                            if let FileStatus::Renamed { old_path } = &file.status {
                                ui.label(
                                    RichText::new(format!("← {}", old_path))
                                        .color(COLOR_META)
                                        .size(11.0),
                                );
                            }
                        });
                    })
                    .response;

                if response.interact(egui::Sense::click()).clicked() {
                    clicked_file = Some(idx);
                }
            }

            if let Some(idx) = clicked_file {
                state.selected_file = Some(idx);
                state.needs_file_load = true;
                state.diff_hunks.clear();
            }
        });
}

fn show_diff_view(ui: &mut Ui, state: &mut AppState) {
    let Some(file_idx) = state.selected_file else {
        ui.centered_and_justified(|ui| {
            ui.label(
                RichText::new("ファイルを選択してください")
                    .color(Color32::from_gray(160))
                    .size(13.0),
            );
        });
        return;
    };

    let is_binary = state
        .diff_files
        .get(file_idx)
        .map(|f| f.is_binary)
        .unwrap_or(false);

    if is_binary {
        ui.centered_and_justified(|ui| {
            ui.label(
                RichText::new("バイナリファイルのため差分を表示できません")
                    .color(COLOR_META)
                    .size(13.0),
            );
        });
        return;
    }

    if state.diff_hunks.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(
                RichText::new("差分なし")
                    .color(Color32::from_gray(160))
                    .size(13.0),
            );
        });
        return;
    }

    egui::ScrollArea::both()
        .id_salt("diff_view_scroll")

        .show(ui, |ui| {
            for hunk in &state.diff_hunks {
                egui::Frame::new()
                    .fill(COLOR_HUNK_HEADER_BG)
                    .inner_margin(egui::Margin::symmetric(6, 2))
                    .show(ui, |ui| {

                        ui.label(
                            RichText::new(&hunk.header)
                                .color(COLOR_HUNK_HEADER_TEXT)
                                .monospace()
                                .size(12.0),
                        );
                    });

                for line in &hunk.lines {
                    let (bg, text_color, prefix) = match line.kind {
                        DiffLineKind::Added => (COLOR_ADDED_BG, COLOR_ADDED_TEXT, "+"),
                        DiffLineKind::Deleted => (COLOR_DELETED_BG, COLOR_DELETED_TEXT, "-"),
                        DiffLineKind::Context => (COLOR_CONTEXT_BG, Color32::DARK_GRAY, " "),
                    };
                    egui::Frame::new()
                        .fill(bg)
                        .inner_margin(egui::Margin::symmetric(6, 1))
                        .show(ui, |ui| {
    
                            ui.label(
                                RichText::new(format!("{}{}", prefix, line.content))
                                    .color(text_color)
                                    .monospace()
                                    .size(12.0),
                            );
                        });
                }

                ui.add_space(4.0);
            }
        });
}
