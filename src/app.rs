use std::path::PathBuf;

use crate::{
    config::{load_config, save_config, AppConfig},
    git::{CommitInfo, DiffFile, DiffHunk, GitRepository},
    ui::{commit_list::show_commit_list, diff_panel::show_diff_panel, toolbar::show_toolbar},
};

const COMMIT_LIMIT: usize = 1000;

pub struct App {
    pub state: AppState,
    repo: Option<GitRepository>,
}

pub struct AppState {
    pub repo_path: Option<PathBuf>,
    pub path_input: String,
    pub commits: Vec<CommitInfo>,
    pub selected_commit: Option<usize>,
    pub diff_files: Vec<DiffFile>,
    pub selected_file: Option<usize>,
    pub diff_hunks: Vec<DiffHunk>,
    pub needs_load: bool,
    pub needs_diff_load: bool,
    pub needs_file_load: bool,
    pub error_message: Option<String>,
    pub split_x: f32,
}

impl AppState {
    fn new() -> Self {
        let config = load_config();
        let (repo_path, path_input, needs_load) = if let Some(p) = config.last_repo_path {
            let path = PathBuf::from(&p);
            (Some(path), p, true)
        } else {
            (None, String::new(), false)
        };

        Self {
            repo_path,
            path_input,
            commits: Vec::new(),
            selected_commit: None,
            diff_files: Vec::new(),
            selected_file: None,
            diff_hunks: Vec::new(),
            needs_load,
            needs_diff_load: false,
            needs_file_load: false,
            error_message: None,
            split_x: 380.0,
        }
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_japanese_font(&cc.egui_ctx);
        Self {
            state: AppState::new(),
            repo: None,
        }
    }

    fn load_repo(&mut self) {
        let path = PathBuf::from(self.state.path_input.trim());
        match GitRepository::open(&path) {
            Ok(repo) => match repo.load_commits(COMMIT_LIMIT) {
                Ok(commits) => {
                    self.state.repo_path = Some(path.clone());
                    self.state.commits = commits;
                    self.state.selected_commit = None;
                    self.state.diff_files = Vec::new();
                    self.state.selected_file = None;
                    self.state.diff_hunks = Vec::new();
                    self.state.error_message = None;
                    save_config(&AppConfig {
                        last_repo_path: Some(path.to_string_lossy().to_string()),
                    });
                    self.repo = Some(repo);
                }
                Err(e) => {
                    self.state.error_message = Some(e.to_string());
                    self.state.commits = Vec::new();
                    self.repo = None;
                }
            },
            Err(e) => {
                self.state.error_message = Some(e.to_string());
                self.state.commits = Vec::new();
                self.repo = None;
            }
        }
    }

    fn load_diff_files(&mut self) {
        let Some(idx) = self.state.selected_commit else {
            return;
        };
        let Some(oid) = self.state.commits.get(idx).map(|c| c.oid.clone()) else {
            return;
        };
        let Some(repo) = &self.repo else {
            return;
        };
        match repo.load_diff_files(&oid) {
            Ok(files) => {
                self.state.diff_files = files;
                self.state.selected_file = None;
                self.state.diff_hunks = Vec::new();
            }
            Err(e) => {
                self.state.error_message = Some(e.to_string());
                self.state.diff_files = Vec::new();
            }
        }
    }

    fn load_diff_hunks(&mut self) {
        let Some(commit_idx) = self.state.selected_commit else {
            return;
        };
        let Some(file_idx) = self.state.selected_file else {
            return;
        };
        let Some(oid) = self.state.commits.get(commit_idx).map(|c| c.oid.clone()) else {
            return;
        };
        let Some(file) = self.state.diff_files.get(file_idx) else {
            return;
        };
        let file_path = file.path.clone();
        let is_binary = file.is_binary;
        if is_binary {
            self.state.diff_hunks = Vec::new();
            return;
        }
        let Some(repo) = &self.repo else {
            return;
        };
        match repo.load_diff_hunks(&oid, &file_path) {
            Ok(hunks) => {
                self.state.diff_hunks = hunks;
            }
            Err(e) => {
                self.state.error_message = Some(e.to_string());
                self.state.diff_hunks = Vec::new();
            }
        }
    }
}

fn setup_japanese_font(ctx: &egui::Context) {
    // Windows システムフォントを優先順に試して日本語グリフを追加する
    let candidates = [
        "C:/Windows/Fonts/meiryo.ttc",
        "C:/Windows/Fonts/YuGothR.ttc",
        "C:/Windows/Fonts/msgothic.ttc",
    ];

    for path in &candidates {
        if let Ok(data) = std::fs::read(path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts
                .font_data
                .insert("jp".to_owned(), egui::FontData::from_owned(data).into());
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "jp".to_owned());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("jp".to_owned());
            ctx.set_fonts(fonts);
            return;
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.state.needs_load {
            self.state.needs_load = false;
            self.load_repo();
        }

        if self.state.needs_diff_load {
            self.state.needs_diff_load = false;
            self.load_diff_files();
        }

        if self.state.needs_file_load {
            self.state.needs_file_load = false;
            self.load_diff_hunks();
        }

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            show_toolbar(ui, &mut self.state);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_rect_before_wrap();
            let min_side = 150.0_f32;
            let split = self.state.split_x
                .max(min_side)
                .min(available.width() - min_side);
            let split_screen_x = available.left() + split;

            // ドラッグ可能なセパレータ
            let sep_rect = egui::Rect::from_min_max(
                egui::pos2(split_screen_x - 4.0, available.top()),
                egui::pos2(split_screen_x + 4.0, available.bottom()),
            );
            let sep_resp = ui.interact(
                sep_rect,
                ui.id().with("split_sep"),
                egui::Sense::drag(),
            );
            if sep_resp.dragged() {
                self.state.split_x = (split + sep_resp.drag_delta().x)
                    .max(min_side)
                    .min(available.width() - min_side);
            }
            let _ = sep_resp.on_hover_cursor(egui::CursorIcon::ResizeHorizontal);

            // セパレータの線
            ui.painter().vline(
                split_screen_x,
                available.top()..=available.bottom(),
                egui::Stroke::new(1.0, egui::Color32::from_gray(210)),
            );

            // 左: コミット一覧
            let left_rect = egui::Rect::from_min_max(
                available.min,
                egui::pos2(split_screen_x - 4.0, available.bottom()),
            );
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(left_rect), |ui| {
                ui.set_clip_rect(left_rect.intersect(ui.clip_rect()));
                show_commit_list(ui, &mut self.state);
            });

            // 右: diff パネル
            let right_rect = egui::Rect::from_min_max(
                egui::pos2(split_screen_x + 4.0, available.top()),
                available.max,
            );
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(right_rect), |ui| {
                ui.set_clip_rect(right_rect.intersect(ui.clip_rect()));
                show_diff_panel(ui, &mut self.state);
            });
        });

        let mut close_error = false;
        if let Some(msg) = &self.state.error_message {
            egui::Window::new("エラー")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new(msg).color(egui::Color32::from_rgb(209, 36, 47)),
                    );
                    ui.add_space(8.0);
                    if ui.button("閉じる").clicked() {
                        close_error = true;
                    }
                });
        }
        if close_error {
            self.state.error_message = None;
        }
    }
}
