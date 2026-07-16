use std::path::PathBuf;

use crate::{
    cli::resolve_target,
    config::{load_config, save_config, AppConfig},
    export::{build_export_html, ExportEntry},
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
    /// クリック順で保持する選択中コミットのindex。最大2件(3件目Ctrl+クリックで先頭を追い出すスライディング選択)。
    pub selected_commits: Vec<usize>,
    pub diff_files: Vec<DiffFile>,
    pub selected_file: Option<usize>,
    pub diff_hunks: Vec<DiffHunk>,
    pub needs_load: bool,
    pub needs_diff_load: bool,
    pub needs_file_load: bool,
    pub needs_export: bool,
    pub error_message: Option<String>,
    pub split_x: f32,
    pub diff_split_y: f32,
    /// ドラッグ&ドロップ等から特定ファイル/フォルダを指定した場合の絞り込み対象パス。
    pub file_filter: Option<String>,
}

impl AppState {
    fn new() -> Self {
        let config = load_config();
        let (repo_path, path_input, needs_load) = repo_path_from_config(&config);

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
            needs_export: false,
            error_message: None,
            split_x: 380.0,
            diff_split_y: 160.0,
            file_filter: None,
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

// ドロップされたファイル群から、実パスを持つ先頭の1件だけを取り出す。
// 複数ファイル/フォルダが同時にドロップされても2件目以降は無視する。
fn first_dropped_path(files: &[egui::DroppedFile]) -> Option<PathBuf> {
    files.iter().find_map(|f| f.path.clone())
}

fn short_oid(oid: &str) -> &str {
    &oid[..7.min(oid.len())]
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

    // Explorer からドロップされたパスを解決し、絞り込み対象(必要ならリポジトリ自体)を切り替える。
    fn apply_dropped_path(&mut self, path: PathBuf) {
        match resolve_target(&path) {
            Ok(target) => {
                self.state.path_input = target.repo_root.to_string_lossy().to_string();
                self.state.file_filter = target.file_filter;
                self.state.needs_load = true;
                self.state.error_message = None;
            }
            Err(e) => {
                self.state.error_message = Some(e.to_string());
            }
        }
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

    /// 選択中コミット範囲の全変更ファイルについてhunksを収集し、HTMLとして書き出す。
    /// 保存先はネイティブの保存ダイアログでユーザーに選ばせる。キャンセル時は何もしない。
    fn export_diff_html(&mut self) {
        let Some((base_oid, target_oid)) = self.selected_diff_oids() else {
            return;
        };
        let Some(repo) = &self.repo else {
            return;
        };

        let mut entries = Vec::with_capacity(self.state.diff_files.len());
        for file in &self.state.diff_files {
            if file.is_binary {
                entries.push(ExportEntry::new(file.clone(), Vec::new()));
                continue;
            }
            let result = match &target_oid {
                Some(target_oid) => repo.load_diff_hunks_between(&base_oid, target_oid, &file.path),
                None => repo.load_diff_hunks(&base_oid, &file.path),
            };
            match result {
                Ok(hunks) => entries.push(ExportEntry::new(file.clone(), hunks)),
                Err(e) => {
                    // 1ファイルでも読み込みに失敗したら、不完全なHTMLを書き出さず全体を中断する。
                    self.state.error_message = Some(e.to_string());
                    return;
                }
            }
        }

        let html = build_export_html(&entries);

        let default_name = match &target_oid {
            Some(target_oid) => format!(
                "diff-{}-{}.html",
                short_oid(&base_oid),
                short_oid(target_oid)
            ),
            None => format!("diff-{}.html", short_oid(&base_oid)),
        };

        let Some(path) = rfd::FileDialog::new()
            .set_file_name(&default_name)
            .add_filter("HTML", &["html"])
            .save_file()
        else {
            return;
        };

        if let Err(e) = std::fs::write(&path, html) {
            self.state.error_message = Some(format!("エクスポートに失敗しました: {}", e));
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
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        if let Some(path) = first_dropped_path(&dropped_files) {
            self.apply_dropped_path(path);
        }

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

        if self.state.needs_export {
            self.state.needs_export = false;
            self.export_diff_html();
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
    use std::fs;
    use std::path::Path;

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

    fn test_state() -> AppState {
        AppState {
            repo_path: None,
            path_input: String::new(),
            commits: Vec::new(),
            selected_commits: Vec::new(),
            diff_files: Vec::new(),
            selected_file: None,
            diff_hunks: Vec::new(),
            needs_load: false,
            needs_diff_load: false,
            needs_file_load: false,
            needs_export: false,
            error_message: None,
            split_x: 380.0,
            diff_split_y: 160.0,
            file_filter: None,
        }
    }

    fn test_app() -> App {
        App {
            state: test_state(),
            repo: None,
        }
    }

    fn dropped_file(path: Option<PathBuf>) -> egui::DroppedFile {
        egui::DroppedFile {
            path,
            ..Default::default()
        }
    }

    fn init_repo_with_commit(dir: &Path, file_name: &str) {
        let repo = git2::Repository::init(dir).unwrap();
        fs::write(dir.join(file_name), "content").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new(file_name)).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = git2::Signature::now("tester", "tester@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();
    }

    #[test]
    fn first_dropped_path_returns_none_for_empty_list() {
        assert_eq!(first_dropped_path(&[]), None);
    }

    #[test]
    fn first_dropped_path_skips_entries_without_a_path() {
        let files = [dropped_file(None)];
        assert_eq!(first_dropped_path(&files), None);
    }

    #[test]
    fn first_dropped_path_returns_the_first_entry_and_ignores_the_rest() {
        let first = PathBuf::from("C:/repo/src/main.rs");
        let second = PathBuf::from("C:/repo/src/lib.rs");
        let files = [
            dropped_file(Some(first.clone())),
            dropped_file(Some(second)),
        ];
        assert_eq!(first_dropped_path(&files), Some(first));
    }

    #[test]
    fn first_dropped_path_skips_leading_none_and_returns_next_valid_path() {
        let valid = PathBuf::from("C:/repo/src/main.rs");
        let files = [dropped_file(None), dropped_file(Some(valid.clone()))];
        assert_eq!(first_dropped_path(&files), Some(valid));
    }

    #[test]
    fn apply_dropped_path_sets_file_filter_for_subfolder() {
        let tmp = std::env::temp_dir().join(format!(
            "gitwit-app-test-subdir-{}-{}",
            std::process::id(),
            "dropfilter"
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("src")).unwrap();
        init_repo_with_commit(&tmp, "src/main.rs");

        let mut app = test_app();
        app.apply_dropped_path(tmp.join("src"));

        assert_eq!(app.state.file_filter.as_deref(), Some("src"));
        assert!(app.state.needs_load);
        assert!(app.state.error_message.is_none());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn apply_dropped_path_clears_file_filter_for_repo_root() {
        let tmp = std::env::temp_dir().join(format!(
            "gitwit-app-test-root-{}-{}",
            std::process::id(),
            "dropfilter"
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        init_repo_with_commit(&tmp, "readme.txt");

        let mut app = test_app();
        app.apply_dropped_path(tmp.clone());

        assert!(app.state.file_filter.is_none());
        assert!(app.state.needs_load);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn apply_dropped_path_sets_error_and_keeps_state_for_path_outside_repo() {
        let tmp = std::env::temp_dir().join(format!(
            "gitwit-app-test-norepo-{}-{}",
            std::process::id(),
            "dropfilter"
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let mut app = test_app();
        app.state.path_input = "previous".to_string();
        app.state.file_filter = Some("previous/filter".to_string());

        app.apply_dropped_path(tmp.clone());

        assert!(app.state.error_message.is_some());
        assert_eq!(app.state.path_input, "previous");
        assert_eq!(app.state.file_filter.as_deref(), Some("previous/filter"));
        assert!(!app.state.needs_load);

        let _ = fs::remove_dir_all(&tmp);
    }
}
