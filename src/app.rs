use std::path::PathBuf;

use crate::{
    cli::resolve_target,
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
    /// クリック順で保持する選択中コミットのindex。最大2件(3件目Shift+クリックで先頭を追い出すスライディング選択)。
    pub selected_commits: Vec<usize>,
    pub diff_files: Vec<DiffFile>,
    pub selected_file: Option<usize>,
    pub diff_hunks: Vec<DiffHunk>,
    pub needs_load: bool,
    pub needs_diff_load: bool,
    pub needs_file_load: bool,
    pub error_message: Option<String>,
    pub split_x: f32,
    pub diff_split_y: f32,
    /// Explorer 右クリック等から特定ファイルを指定して起動した場合の絞り込み対象パス。
    pub file_filter: Option<String>,
}

impl AppState {
    fn new(cli_target: Option<PathBuf>) -> Self {
        let config = load_config();

        let mut error_message = None;
        let mut file_filter = None;

        let (repo_path, path_input, needs_load) = match cli_target.as_deref().map(resolve_target) {
            Some(Ok(target)) => {
                file_filter = target.file_filter;
                let path_input = target.repo_root.to_string_lossy().to_string();
                (Some(target.repo_root), path_input, true)
            }
            Some(Err(e)) => {
                error_message = Some(e.to_string());
                repo_path_from_config(&config)
            }
            None => repo_path_from_config(&config),
        };

        Self {
            repo_path,
            path_input,
            commits: Vec::new(),
            selected_commits: Vec::new(),
            diff_files: Vec::new(),
            selected_file: None,
            diff_hunks: Vec::new(),
            needs_load,
            needs_diff_load: false,
            needs_file_load: false,
            error_message,
            split_x: 380.0,
            diff_split_y: 160.0,
            file_filter,
        }
    }
}

/// `selected` の長さに応じて、比較対象のoidを決定する。
/// 2件選択時は `commits` 内でのindex(revwalkのTIMEソート順、0が最新)を履歴順の正として、
/// indexが大きい方(古い)をbase、小さい方(新しい)をtargetとする。
fn resolve_diff_oids(
    commits: &[CommitInfo],
    selected: &[usize],
) -> Option<(String, Option<String>)> {
    match selected {
        [idx] => commits.get(*idx).map(|c| (c.oid.clone(), None)),
        [idx_a, idx_b] => {
            let a = commits.get(*idx_a)?;
            let b = commits.get(*idx_b)?;
            let (base, target) = if idx_a > idx_b { (a, b) } else { (b, a) };
            Some((base.oid.clone(), Some(target.oid.clone())))
        }
        _ => None,
    }
}

fn repo_path_from_config(config: &AppConfig) -> (Option<PathBuf>, String, bool) {
    if let Some(p) = config.last_repo_path.clone() {
        let path = PathBuf::from(&p);
        (Some(path), p, true)
    } else {
        (None, String::new(), false)
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, cli_target: Option<PathBuf>) -> Self {
        setup_japanese_font(&cc.egui_ctx);
        Self {
            state: AppState::new(cli_target),
            repo: None,
        }
    }

    fn load_repo(&mut self) {
        let path = PathBuf::from(self.state.path_input.trim());
        match GitRepository::open(&path) {
            Ok(repo) => match self.load_commits_for_state(&repo) {
                Ok(commits) => {
                    self.state.repo_path = Some(path.clone());
                    self.state.commits = commits;
                    self.state.selected_commits.clear();
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

    fn load_commits_for_state(
        &self,
        repo: &GitRepository,
    ) -> Result<Vec<CommitInfo>, crate::git::GitError> {
        match &self.state.file_filter {
            Some(path) => repo.load_commits_for_path(COMMIT_LIMIT, path),
            None => repo.load_commits(COMMIT_LIMIT),
        }
    }

    /// 選択中コミットのoidを、選択が2件の場合は履歴順(古い→新しい)で(base, target)として返す。
    /// 選択が1件の場合はそのコミットのoidのみを返す。
    fn selected_diff_oids(&self) -> Option<(String, Option<String>)> {
        resolve_diff_oids(&self.state.commits, &self.state.selected_commits)
    }

    fn load_diff_files(&mut self) {
        let Some((base_oid, target_oid)) = self.selected_diff_oids() else {
            return;
        };
        let Some(repo) = &self.repo else {
            return;
        };
        let result = match &target_oid {
            Some(target_oid) => repo.load_diff_files_between(&base_oid, target_oid),
            None => repo.load_diff_files(&base_oid),
        };
        match result {
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
        let Some((base_oid, target_oid)) = self.selected_diff_oids() else {
            return;
        };
        let Some(file_idx) = self.state.selected_file else {
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
        let result = match &target_oid {
            Some(target_oid) => repo.load_diff_hunks_between(&base_oid, target_oid, &file_path),
            None => repo.load_diff_hunks(&base_oid, &file_path),
        };
        match result {
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
            let split = self
                .state
                .split_x
                .max(min_side)
                .min(available.width() - min_side);
            let split_screen_x = available.left() + split;

            // ドラッグ可能なセパレータ
            let sep_rect = egui::Rect::from_min_max(
                egui::pos2(split_screen_x - 4.0, available.top()),
                egui::pos2(split_screen_x + 4.0, available.bottom()),
            );
            let sep_resp = ui.interact(sep_rect, ui.id().with("split_sep"), egui::Sense::drag());
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
                    ui.label(egui::RichText::new(msg).color(egui::Color32::from_rgb(209, 36, 47)));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(oid: &str, time: i64) -> CommitInfo {
        CommitInfo {
            oid: oid.to_string(),
            short_id: oid[..7.min(oid.len())].to_string(),
            message: String::new(),
            author: String::new(),
            time,
            refs: Vec::new(),
        }
    }

    #[test]
    fn resolve_diff_oids_single_selection_returns_oid_only() {
        let commits = vec![commit("aaaaaaa", 100)];
        assert_eq!(
            resolve_diff_oids(&commits, &[0]),
            Some(("aaaaaaa".to_string(), None))
        );
    }

    #[test]
    fn resolve_diff_oids_two_selection_orders_by_index_newer_first() {
        // commits はrevwalk(TIMEソート)順、index 0 が最新
        let commits = vec![commit("newer", 200), commit("older", 100)];
        assert_eq!(
            resolve_diff_oids(&commits, &[0, 1]),
            Some(("older".to_string(), Some("newer".to_string())))
        );
        // クリック順を入れ替えても結果は変わらない
        assert_eq!(
            resolve_diff_oids(&commits, &[1, 0]),
            Some(("older".to_string(), Some("newer".to_string())))
        );
    }

    #[test]
    fn resolve_diff_oids_two_selection_same_second_uses_index_not_time() {
        // 同一秒にコミットされた場合でも、indexの新旧関係を優先する
        let commits = vec![commit("newer", 100), commit("older", 100)];
        assert_eq!(
            resolve_diff_oids(&commits, &[0, 1]),
            Some(("older".to_string(), Some("newer".to_string())))
        );
    }

    #[test]
    fn resolve_diff_oids_no_selection_returns_none() {
        let commits = vec![commit("aaaaaaa", 100)];
        assert_eq!(resolve_diff_oids(&commits, &[]), None);
    }
}
