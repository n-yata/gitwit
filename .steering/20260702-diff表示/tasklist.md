# タスクリスト: diff表示

## タスク

- [x] T1: `src/git/diff.rs` を新規作成（型定義 + load 関数）
- [x] T2: `src/git/mod.rs` に diff モジュールと型を追加
- [x] T3: `src/git/repository.rs` に `load_diff_files` / `load_diff_hunks` を追加
- [x] T4: `src/git/commit.rs` の `#[allow(dead_code)]` を削除
- [x] T5: `src/app.rs` の AppState に diff 関連フィールドを追加、App 構造体に `repo` を追加、load メソッド追加、レイアウトを SidePanel に変更
- [x] T6: `src/ui/mod.rs` に diff_panel モジュールを追加
- [x] T7: `src/ui/commit_list.rs` にクリック時の needs_diff_load フラグ設定を追加
- [x] T8: `src/ui/diff_panel.rs` を新規作成（ファイル一覧 + diff 表示）
- [x] T9: `cargo build` でコンパイル確認
- [x] T10: `cargo test` でテスト実行
- [x] T11: `cargo clippy -- -D warnings` でリント確認
