# タスクリスト

## 🚨 タスク完全完了の原則

**このファイルの全タスクが完了するまで作業を継続すること**

### 必須ルール
- **全てのタスクを`[x]`にすること**
- 「時間の都合により別タスクとして実施予定」は禁止
- 「実装が複雑すぎるため後回し」は禁止
- 未完了タスク（`[ ]`）を残したまま作業を終了しない

### タスクスキップが許可される唯一のケース
以下の技術的理由に該当する場合のみスキップ可能:
- 実装方針の変更により、機能自体が不要になった
- アーキテクチャ変更により、別の実装方法に置き換わった
- 依存関係の変更により、タスクが実行不可能になった

スキップ時は必ず理由を明記:
```markdown
- [x] ~~タスク名~~（実装方針変更により不要: 具体的な技術的理由）
```

---

## フェーズ1: 共通ロジックの集約（git層）

- [x] `src/ui/diff_panel.rs` の `SideCell` / `build_side_by_side_rows` を `src/git/diff.rs` に移設する
  - [x] `git/diff.rs` に `SideCell`/`build_side_by_side_rows` を `pub` として定義（`git::mod`経由で再エクスポート）
  - [x] `diff_panel.rs` 側は `use crate::git::{...}` に切り替え、重複定義を削除
  - [x] 既存テスト・既存の呼び出し箇所（`show_diff_view`）が壊れていないことを確認（`cargo build`成功）
- [x] `src/git/diff.rs` に `count_changed_lines(hunks: &[DiffHunk]) -> (usize, usize)`（追加行数, 削除行数）を追加
  - [x] ユニットテストを追加（Added/Deletedそれぞれの集計が正しいこと）

## フェーズ2: エクスポートモジュール実装

- [x] `Cargo.toml` に `rfd = "0.15"` を追加
- [x] `src/export/html.rs` を新規作成
  - [x] `pub struct ExportEntry { file: DiffFile, hunks: Vec<DiffHunk> }` を定義(行数は`count_changed_lines`で都度算出する方針に変更)
  - [x] `escape_html(s: &str) -> String` を実装（`&`, `<`, `>`, `"`, `'` をエスケープ）
  - [x] `pub fn build_export_html(entries: &[ExportEntry]) -> String` を実装
    - [x] 変更ファイル一覧テーブル（パス・変更種別バッジ・+N/-N・リネーム旧パス）を生成
    - [x] 各ファイルの差分をside-by-sideで`<div id="diff-{idx}">`に埋め込み（`build_side_by_side_rows`を再利用）
    - [x] バイナリファイルは一覧に「(binary)」注記を出し、diff divは「バイナリファイルのため差分を表示できません」のプレースホルダにする
    - [x] 一覧クリックで対応するdiv表示を切り替えるインラインJS（外部リソース参照なし）を埋め込む
    - [x] 配色をdiff_panel.rsに合わせたインライン`<style>`を埋め込む
  - [x] ユニットテストを追加:
    - [x] `escape_html`が`<script>`等を無害化すること
    - [x] バイナリファイルのdiff divにコード内容が出力されないこと
    - [x] 一覧に+N/-Nが正しく表示されること
    - [x] 生成HTMLに全ファイルパス（エスケープ後）が含まれること
- [x] `src/export/mod.rs` を新規作成し `pub mod html; pub use html::{build_export_html, ExportEntry};` を定義
- [x] `src/main.rs` に `mod export;` を追加

## フェーズ3: アプリ統合（UI・App State）

- [x] `AppState` に `pub needs_export: bool` を追加（`AppState::new()`・テスト用`test_state()`にも初期値`false`を追加）
- [x] `src/ui/diff_panel.rs` の `show_file_list()` 手前に「HTMLエクスポート」ボタンを追加
  - [x] `state.diff_files` が空の場合はボタンを無効化（disabled）
  - [x] クリック時に `state.needs_export = true` をセットするのみ（Git/ファイルI/Oには関与しない）
- [x] `src/app.rs` に `export_diff_html(&mut self)` を実装
  - [x] `selected_diff_oids()` で対象範囲を確認（未選択なら早期リターン）
  - [x] `state.diff_files` の各ファイルについて、バイナリでなければ `repo.load_diff_hunks[_between]()` でhunksを取得（行数集計は`export::html`側の`count_changed_lines`呼び出しに一本化）。バイナリなら空hunks
  - [x] `export::build_export_html(&entries)` でHTML文字列を生成
  - [x] デフォルトファイル名（`diff-{short_id}.html` / 2コミット比較時は `diff-{base_short}-{target_short}.html`）を組み立てる
  - [x] `rfd::FileDialog` で保存先を選択させ、キャンセル時は何もせず終了
  - [x] `std::fs::write()` で書き込み、失敗時は `state.error_message` にセット
- [x] `App::update()` のイベントループに `needs_export` の処理分岐を追加（`needs_load`等と同じパターン）

## フェーズ4: 品質チェックと修正

- [x] すべてのテストが通ることを確認
  - [x] `cargo test`（38 passed）
- [x] リント・フォーマットに問題がないことを確認
  - [x] `cargo clippy --all-targets -- -D warnings`（エラーなし）
  - [x] `cargo fmt --check`（本タスクで変更したファイルのみ確認。`cli.rs`/`git/commit.rs`はrustfmtバージョン差による既存の未整形箇所で、本機能とは無関係のため対象外）
- [x] ビルドが成功することを確認
  - [x] `cargo build`

## フェーズ5: ドキュメント更新

- [x] `docs/architecture.md` の依存関係管理表に `rfd` を追加
- [x] ~~README.md を更新~~（スキップ: リポジトリ直下にREADME.mdが存在しないため。`.claude/README.md`はcommand/skill/agentカタログで本機能とは無関係）
- [x] 実装後の振り返りを記録（別ファイル `retrospective.md` に記録 → モード3）

## フェーズ6: implementation-validatorレビュー対応

- [x] エラーreturn箇所（`app.rs::export_diff_html`）に「1ファイルでも読み込み失敗したら全体を中断する」旨のコメントを追加
- [x] リネーム+バイナリの組み合わせテストケースを `src/export/html.rs` に追加
- [x] `cargo test`（39 passed）/ `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check`（変更ファイルのみ）を再確認

---

> **振り返りについて**: 実装後の振り返りはこのファイルではなく、同じディレクトリの
> `retrospective.md` に記録する（テンプレート: `.claude/skills/steering/templates/retrospective.md`）。
> 全タスクが `[x]` になったことを確認してから作成すること。
