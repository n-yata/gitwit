# 設計書: diff表示

## アーキテクチャ概要

```
UI Layer                           Game Logic Layer
─────────────────────────────────────────────────────
src/ui/diff_panel.rs              src/git/diff.rs
  show_diff_panel()      ←─────    DiffFile / DiffHunk / DiffLine / FileStatus
  show_file_list()               src/git/repository.rs
  show_diff_view()                 load_diff_files()
                                   load_diff_hunks()
src/app.rs (AppState)
  diff_files / selected_file
  diff_hunks
  needs_diff_load / needs_file_load
```

## 型定義 (`src/git/diff.rs`)

```rust
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed { old_path: String },
}

pub struct DiffFile {
    pub path: String,
    pub status: FileStatus,
    pub is_binary: bool,
}

pub enum DiffLineKind {
    Added,
    Deleted,
    Context,
}

pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
}

pub struct DiffHunk {
    pub header: String,    // @@ -x,y +a,b @@ 形式
    pub lines: Vec<DiffLine>,
}
```

## git2 API 利用方針

### load_diff_files
1. `oid_str` から Commit を取得
2. parent[0] の tree と current tree を `diff_tree_to_tree` に渡す
3. `diff.foreach()` のファイルコールバックで DiffFile を収集
4. バイナリ: `delta.new_file().is_binary()` で判定

### load_diff_hunks
1. `oid_str` と `file_path` から Diff を作成 (`DiffOptions::pathspec` でファイルを絞る)
2. **`git2::Patch::from_diff(&diff, 0)` を使用** (foreach ではなく Patch API)
   - `patch.num_hunks()` でハンク数
   - `patch.hunk(i)` で (DiffHunk, line_count)
   - `patch.line_in_hunk(hunk_i, line_i)` で各行
3. `line.origin()` で `'+'` / `'-'` / その他 を判定
4. バイナリの場合は空 Vec を返す（UI 側でメッセージ表示）

## AppState 追加フィールド

```rust
pub diff_files: Vec<DiffFile>,
pub selected_file: Option<usize>,
pub diff_hunks: Vec<DiffHunk>,
pub needs_diff_load: bool,    // コミット選択時に true
pub needs_file_load: bool,    // ファイル選択時に true
```

## App 構造体変更

```rust
pub struct App {
    pub state: AppState,
    repo: Option<GitRepository>,  // 毎フレーム open しないためキャッシュ
}
```

- `load_repo()` 成功時に `self.repo = Some(repo)` をセット
- `load_diff_files()` / `load_diff_hunks()` は `self.repo.as_ref()` を使う

## レイアウト変更

```
TopBottomPanel::top("toolbar")         ← 変更なし
SidePanel::left("commit_list_panel")   ← NEW: コミット一覧を左ペインへ
  min_width(280), default_width(380)
CentralPanel::default()                ← diff パネル
  show_diff_panel()
    allocate_ui(height=28%)  →  show_file_list()
    separator
    ScrollArea               →  show_diff_view()
```

## UI 色定数 (diff_panel.rs)

```rust
const COLOR_ADDED_BG: Color32 = Color32::from_rgb(221, 244, 220);   // 緑
const COLOR_DELETED_BG: Color32 = Color32::from_rgb(255, 220, 220); // 赤
const COLOR_CONTEXT_BG: Color32 = Color32::from_rgb(250, 250, 250); // 薄グレー
const COLOR_ADDED_TEXT: Color32 = Color32::from_rgb(0, 100, 0);
const COLOR_DELETED_TEXT: Color32 = Color32::from_rgb(150, 0, 0);
const COLOR_HUNK_HEADER: Color32 = Color32::from_rgb(0, 70, 140);   // 青
const COLOR_FILE_SELECTED: Color32 = Color32::from_rgb(232, 240, 254);
```

## needs_diff_load / needs_file_load フロー

```
commit_list.rs: clicked → state.selected_commit = idx
                          state.needs_diff_load = true
                          state.diff_files.clear()
                          state.selected_file = None
                          state.diff_hunks.clear()

app.rs update(): needs_diff_load → self.load_diff_files()
                 needs_file_load → self.load_diff_hunks()

diff_panel.rs: file clicked → state.selected_file = idx
                               state.needs_file_load = true
                               state.diff_hunks.clear()
```

## 変更ファイル一覧

| ファイル | 変更種別 |
|---------|---------|
| `src/git/diff.rs` | 新規作成 |
| `src/git/mod.rs` | 型・関数の re-export 追加 |
| `src/git/repository.rs` | load_diff_files / load_diff_hunks 追加 |
| `src/git/commit.rs` | `#[allow(dead_code)]` 削除 |
| `src/app.rs` | AppState フィールド追加、App 構造体変更、レイアウト変更 |
| `src/ui/mod.rs` | diff_panel モジュール追加 |
| `src/ui/commit_list.rs` | needs_diff_load フラグ設定追加 |
| `src/ui/diff_panel.rs` | 新規作成 |
