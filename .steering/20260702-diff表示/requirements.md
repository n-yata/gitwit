# 要求定義: diff表示

## 概要

コミット履歴表示の続き。MVP 完成のための最終機能。
コミットを選択したときに右ペインに変更ファイル一覧と diff を表示する。

## 実装スコープ

### 含む
- `DiffFile`・`DiffHunk`・`DiffLine`・`FileStatus` 型定義（`src/git/diff.rs`）
- `load_diff_files(oid)`: コミットの変更ファイル一覧取得
- `load_diff_hunks(oid, path)`: 特定ファイルの diff 取得（git2 Patch API 使用）
- `GitRepository` に `load_diff_files` / `load_diff_hunks` を追加
- `App` 構造体に `repo: Option<GitRepository>` を保持（毎回 open しない）
- `AppState` に `diff_files`, `selected_file`, `diff_hunks`, `needs_diff_load`, `needs_file_load` を追加
- レイアウト変更: `SidePanel::left`（コミット一覧）+ `CentralPanel`（diff パネル）
- `src/ui/diff_panel.rs`: 変更ファイル一覧（上 28%）+ diff 表示（下 72%）
- バイナリファイルは「バイナリファイルのため差分を表示できません」を表示
- `CommitInfo.oid` の `#[allow(dead_code)]` を削除（今回使用）

### 含まない
- 1MB 超ファイルのサイズチェック（次フェーズ）
- シンタックスハイライト（次フェーズ）

## 受け入れ条件

- [ ] コミットをクリックすると変更ファイル一覧が右上に表示される
- [ ] ファイルをクリックすると diff が右下に表示される
- [ ] 追加行は緑背景、削除行は赤背景、コンテキスト行はグレー背景で表示
- [ ] バイナリファイルはプレースホルダを表示する
- [ ] `cargo build` / `cargo test` / `cargo clippy -- -D warnings` が全て通る
