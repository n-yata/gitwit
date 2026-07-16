use egui::{Color32, RichText, Ui};

use crate::{
    app::AppState,
    git::{build_side_by_side_rows, DiffFile, DiffLineKind, FileStatus, SideCell},
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
const COLOR_EMPTY_BG: Color32 = Color32::from_rgb(240, 240, 240);
const COLOR_FILTER_TARGET: Color32 = Color32::from_rgb(230, 81, 0);
const COLOR_FILTER_TARGET_BG: Color32 = Color32::from_rgb(255, 243, 224);

/// パス区切りを `/` に正規化する（Windowsの `\` 混在パス対策）。
fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

/// `file` が `file_filter` で絞り込まれた対象（ファイル本体 or そのフォルダ配下）かどうかを判定する。
/// リネームされたファイルは旧パスも判定対象に含める。
fn file_matches_filter(file: &DiffFile, filter: &str) -> bool {
    let filter = normalize_path(filter);
    let matches_path = |path: &str| {
        let path = normalize_path(path);
        path == filter || path.starts_with(&format!("{}/", filter))
    };
    if matches_path(&file.path) {
        return true;
    }
    if let FileStatus::Renamed { old_path } = &file.status {
        if matches_path(old_path.as_str()) {
            return true;
        }
    }
    false
}

fn render_side_cell(ui: &mut Ui, cell: &SideCell<'_>) {
    ui.set_width(ui.available_width());
    match cell {
        SideCell::Empty => {
            egui::Frame::new()
                .fill(COLOR_EMPTY_BG)
                .inner_margin(egui::Margin::symmetric(6, 1))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(" ");
                });
        }
        SideCell::Line(line) => {
            let (bg, text_color, prefix) = match line.kind {
                DiffLineKind::Added => (COLOR_ADDED_BG, COLOR_ADDED_TEXT, "+"),
                DiffLineKind::Deleted => (COLOR_DELETED_BG, COLOR_DELETED_TEXT, "-"),
                DiffLineKind::Context => (COLOR_CONTEXT_BG, Color32::DARK_GRAY, " "),
            };
            egui::Frame::new()
                .fill(bg)
                .inner_margin(egui::Margin::symmetric(6, 1))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    // 折り返しを許すと長い行だけ複数行になり、左右ペインの行がズレる。
                    // 常に1行に固定し、はみ出た分は外側の ScrollArea::both() の横スクロールに委ねる。
                    ui.add(
                        egui::Label::new(
                            RichText::new(format!("{}{}", prefix, line.content))
                                .color(text_color)
                                .monospace()
                                .size(12.0),
                        )
                        .wrap_mode(egui::TextWrapMode::Extend),
                    );
                });
        }
    }
}

pub fn show_diff_panel(ui: &mut Ui, state: &mut AppState) {
    if state.selected_commits.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(
                RichText::new("コミットを選択してください")
                    .color(Color32::from_gray(150))
                    .size(16.0),
            );
        });
        return;
    }

    let available = ui.available_rect_before_wrap();
    let min_pane = 60.0_f32;
    let file_list_height = state
        .diff_split_y
        .max(min_pane)
        .min(available.height() - min_pane);
    let sep_screen_y = available.top() + file_list_height;

    // ドラッグ可能なセパレータ
    let sep_rect = egui::Rect::from_min_max(
        egui::pos2(available.left(), sep_screen_y - 4.0),
        egui::pos2(available.right(), sep_screen_y + 4.0),
    );
    let sep_resp = ui.interact(
        sep_rect,
        ui.id().with("diff_split_sep"),
        egui::Sense::drag(),
    );
    if sep_resp.dragged() {
        state.diff_split_y = (file_list_height + sep_resp.drag_delta().y)
            .max(min_pane)
            .min(available.height() - min_pane);
    }
    let _ = sep_resp.on_hover_cursor(egui::CursorIcon::ResizeVertical);

    ui.painter().hline(
        available.left()..=available.right(),
        sep_screen_y,
        egui::Stroke::new(1.0, Color32::from_gray(210)),
    );

    // 上: 変更ファイル一覧
    let top_rect = egui::Rect::from_min_max(
        available.min,
        egui::pos2(available.right(), sep_screen_y - 4.0),
    );
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(top_rect), |ui| {
        ui.set_clip_rect(top_rect.intersect(ui.clip_rect()));
        show_file_list(ui, state);
    });

    // 下: コード差分
    let bottom_rect = egui::Rect::from_min_max(
        egui::pos2(available.left(), sep_screen_y + 4.0),
        available.max,
    );
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(bottom_rect), |ui| {
        ui.set_clip_rect(bottom_rect.intersect(ui.clip_rect()));
        show_diff_view(ui, state);
    });
}

fn show_file_list(ui: &mut Ui, state: &mut AppState) {
    let has_files = !state.diff_files.is_empty();
    ui.add_enabled_ui(has_files, |ui| {
        if ui.button("HTMLエクスポート").clicked() {
            state.needs_export = true;
        }
    });

    if !has_files {
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
                let is_filter_target = state
                    .file_filter
                    .as_deref()
                    .is_some_and(|filter| file_matches_filter(file, filter));
                let bg = if is_selected {
                    COLOR_FILE_SELECTED
                } else if is_filter_target {
                    COLOR_FILTER_TARGET_BG
                } else {
                    Color32::TRANSPARENT
                };

                let mut frame = egui::Frame::new()
                    .fill(bg)
                    .inner_margin(egui::Margin::symmetric(6, 3));
                if is_filter_target {
                    frame = frame.stroke(egui::Stroke::new(1.0, COLOR_FILTER_TARGET));
                }

                let response = frame
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

                            let path_text = RichText::new(&file.path).monospace().size(12.0);
                            let path_text = if is_filter_target {
                                path_text.color(COLOR_FILTER_TARGET).strong()
                            } else {
                                path_text
                            };
                            ui.add(egui::Label::new(path_text).truncate());

                            if file.is_binary {
                                ui.label(RichText::new("(binary)").color(COLOR_META).size(11.0));
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
            for (hunk_idx, hunk) in state.diff_hunks.iter().enumerate() {
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

                let rows = build_side_by_side_rows(&hunk.lines);

                ui.push_id(hunk_idx, |ui| {
                    ui.columns(2, |columns| {
                        for (left, right) in &rows {
                            render_side_cell(&mut columns[0], left);
                            render_side_cell(&mut columns[1], right);
                        }
                    });
                });

                ui.add_space(4.0);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, status: FileStatus) -> DiffFile {
        DiffFile {
            path: path.to_string(),
            status,
            is_binary: false,
        }
    }

    #[test]
    fn file_matches_filter_exact_path_match() {
        let f = file("src/app.rs", FileStatus::Modified);
        assert!(file_matches_filter(&f, "src/app.rs"));
    }

    #[test]
    fn file_matches_filter_directory_prefix_match() {
        let f = file("src/ui/diff_panel.rs", FileStatus::Modified);
        assert!(file_matches_filter(&f, "src/ui"));
    }

    #[test]
    fn file_matches_filter_rejects_unrelated_file() {
        let f = file("src/ui/toolbar.rs", FileStatus::Modified);
        assert!(!file_matches_filter(&f, "src/app.rs"));
    }

    #[test]
    fn file_matches_filter_rejects_sibling_with_shared_prefix() {
        // "src/app" は "src/app.rs" のディレクトリ配下ではないため一致しない
        let f = file("src/app.rs", FileStatus::Modified);
        assert!(!file_matches_filter(&f, "src/ap"));
    }

    #[test]
    fn file_matches_filter_matches_renamed_old_path() {
        let f = file(
            "src/new_name.rs",
            FileStatus::Renamed {
                old_path: "src/old_name.rs".to_string(),
            },
        );
        assert!(file_matches_filter(&f, "src/old_name.rs"));
        assert!(file_matches_filter(&f, "src/new_name.rs"));
    }

    #[test]
    fn file_matches_filter_normalizes_windows_path_separators() {
        let f = file("src\\ui\\diff_panel.rs", FileStatus::Modified);
        assert!(file_matches_filter(&f, "src/ui"));
    }
}
