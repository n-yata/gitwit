use egui::{Color32, RichText, Ui};

use crate::{app::AppState, git::format_relative_time};

const COLOR_SELECTED: Color32 = Color32::from_rgb(232, 240, 254);
const COLOR_BRANCH_BG: Color32 = Color32::from_rgb(0, 117, 202);
const COLOR_TAG_BG: Color32 = Color32::from_rgb(233, 155, 0);
const COLOR_HASH: Color32 = Color32::from_rgb(100, 100, 100);
const COLOR_META: Color32 = Color32::from_rgb(130, 130, 130);

pub fn show_commit_list(ui: &mut Ui, state: &mut AppState) {
    if state.commits.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(
                RichText::new("リポジトリを開いてください")
                    .color(Color32::from_gray(150))
                    .size(16.0),
            );
        });
        return;
    }

    egui::ScrollArea::vertical()
        .show(ui, |ui| {
            let mut clicked_idx: Option<usize> = None;

            for (idx, commit) in state.commits.iter().enumerate() {
                let is_selected = state.selected_commit == Some(idx);
                let bg = if is_selected {
                    COLOR_SELECTED
                } else {
                    Color32::TRANSPARENT
                };

                let response = egui::Frame::new()
                    .fill(bg)
                    .inner_margin(egui::Margin::symmetric(6, 4))
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            // 1行目: ハッシュ + メッセージ + バッジ
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&commit.short_id)
                                            .color(COLOR_HASH)
                                            .monospace()
                                            .size(12.0),
                                    )
                                    .truncate(),
                                );
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&commit.message).strong().size(13.0),
                                    )
                                    .truncate(),
                                );
                                for ref_name in &commit.refs {
                                    let bg_color = if ref_name.starts_with("tag:") {
                                        COLOR_TAG_BG
                                    } else {
                                        COLOR_BRANCH_BG
                                    };
                                    egui::Frame::new()
                                        .fill(bg_color)
                                        .corner_radius(3)
                                        .inner_margin(egui::Margin::symmetric(4, 1))
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new(ref_name)
                                                    .color(Color32::WHITE)
                                                    .size(11.0),
                                            );
                                        });
                                }
                            });
                            // 2行目: 著者 + 日時
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(format!(
                                            "{}  {}",
                                            commit.author,
                                            format_relative_time(commit.time)
                                        ))
                                        .color(COLOR_META)
                                        .size(11.0),
                                    )
                                    .truncate(),
                                );
                            });
                        });
                    })
                    .response;

                if response.interact(egui::Sense::click()).clicked() {
                    clicked_idx = Some(idx);
                }

                ui.separator();
            }

            if let Some(idx) = clicked_idx {
                state.selected_commit = Some(idx);
                state.needs_diff_load = true;
                state.diff_files.clear();
                state.selected_file = None;
                state.diff_hunks.clear();
            }
        });
}
