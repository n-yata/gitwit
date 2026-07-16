# 設計書

## アーキテクチャ概要

既存のレイヤードアーキテクチャ（UI → App State → Git ロジック）に、新たに **Export レイヤー**（`src/export/`）を追加する。Export レイヤーは Git ロジックレイヤー（`DiffFile`/`DiffHunk`）のみに依存する純粋関数群とし、UI・ファイルI/O には関与しない。ファイル保存（保存ダイアログ表示・書き込み）は App State レイヤー（`src/app.rs`）が担当する。

```
┌──────────────────────────────────────┐
│ UI レイヤー（src/ui/diff_panel.rs）    │
│  「HTMLエクスポート」ボタン追加        │
│  → state.needs_export = true          │
├──────────────────────────────────────┤
│ App State レイヤー（src/app.rs）      │
│  needs_export 検知                     │
│  → 全DiffFileのhunksをrepo経由で収集  │
│  → export::build_export_html() 呼出   │
│  → rfd で保存ダイアログ→ファイル書込  │
├──────────────────────────────────────┤
│ Export レイヤー（src/export/html.rs） │
│  DiffFile/DiffHunk → 自己完結HTML文字列│
│  （純粋関数・ファイルI/Oなし）         │
├──────────────────────────────────────┤
│ Git ロジックレイヤー（src/git/diff.rs）│
│  SideCell / build_side_by_side_rows を │
│  UI・Export 両方から再利用できるよう   │
│  ここに集約（既存はdiff_panel.rs内）  │
└──────────────────────────────────────┘
```

## コンポーネント設計

### 1. `src/git/diff.rs`（既存ファイルの拡張）

**責務**:
- 既存の `DiffFile`/`DiffHunk`/`DiffLine` に加えて、side-by-side 行整形ロジック（`SideCell`/`build_side_by_side_rows`）を `src/ui/diff_panel.rs` からここへ移設し、UI・Export の両方から再利用できる純粋関数にする。
- ファイルごとの追加/削除行数を集計するヘルパー `count_changed_lines(hunks: &[DiffHunk]) -> (usize, usize)`（+N, -N）を追加する。

**実装の要点**:
- `SideCell`/`build_side_by_side_rows` は egui に一切依存しないため、そのまま `git/diff.rs` に移設可能。`diff_panel.rs` 側は `use crate::git::diff::{SideCell, build_side_by_side_rows};` に切り替える。
- 可視性は `pub(crate)` とし、外部クレートからは触れないようにする。

### 2. `src/export/html.rs`（新規）

**責務**:
- `DiffFile` と、そのファイルに対応する `Vec<DiffHunk>`（バイナリの場合は空）のペアのリストを受け取り、自己完結HTML文字列を1つ組み立てる。
- HTML/CSS/JS はすべてインライン埋め込みとし、外部リソースへの参照を一切含めない。

**実装の要点**:
- 公開関数: `pub fn build_export_html(entries: &[ExportEntry]) -> String`
  - `pub struct ExportEntry { pub file: DiffFile, pub hunks: Vec<DiffHunk>, pub added: usize, pub deleted: usize }`
- 一覧テーブル: 各行に `data-file-idx` を持たせ、パス・変更種別バッジ（A/M/D/R、リネームは旧パスも表示）・`+N`/`-N` を表示。
- 各ファイルの差分は `<div id="diff-{idx}" style="display:none">` としてページ内に埋め込み、一覧クリックで対象の `div` だけ表示切り替えする vanilla JS（`<script>` インライン、フレームワーク不使用）。
- バイナリファイル（`file.is_binary`）は一覧行に「(binary)」注記を出し、対応する diff div は「バイナリファイルのため差分を表示できません」という文言のみのプレースホルダにする（hunks は空のはず）。
- side-by-side 描画は `git::diff::build_side_by_side_rows` を再利用し、`SideCell::Line`/`Empty` を HTML の `<div class="cell added|deleted|context|empty">` に変換する。
- **XSS対策（重要）**: ファイルパス・diff本文はユーザーのリポジトリ内容（任意のコード）であり信頼できない入力として扱う。HTML に埋め込む全ての値を独自の `escape_html()` 関数（`&`, `<`, `>`, `"`, `'` をエスケープ）に通してからテンプレートに差し込む。属性値・テキストノードいずれも例外なくエスケープする。
- CSSは `diff_panel.rs` の配色（追加=緑系背景、削除=赤系背景、コンテキスト=薄灰）に合わせた `<style>` をインライン埋め込み。

**テスト対象**:
- `escape_html()` が `<script>` 等を無害化すること
- バイナリファイルの diff div にコード内容が出力されないこと
- 一覧に変更行数（+N/-N）が正しく表示されること

### 3. `src/export/mod.rs`（新規）

- `pub mod html;` と `pub use html::{build_export_html, ExportEntry};` のみ。

### 4. `src/app.rs`（拡張）

**責務**:
- `AppState` に `pub needs_export: bool` を追加（`needs_load` 等と同じパターン）。
- `App::update()` のイベントループに `needs_export` の処理分岐を追加。
- 新規メソッド `fn export_diff_html(&mut self)`:
  1. `selected_diff_oids()` で対象範囲（単一 or 2コミット比較）を確認。未選択なら何もしない。
  2. `state.diff_files` の各ファイルについて、バイナリでなければ `repo.load_diff_hunks[_between]()` を呼んで hunks を収集し、`count_changed_lines()` で行数集計。バイナリなら空の hunks・行数0。
  3. `export::build_export_html(&entries)` でHTML文字列を生成。
  4. `rfd::FileDialog::new().set_file_name(default_name).add_filter("HTML", &["html"]).save_file()` でパスを取得（キャンセル時は何もしない）。
  5. `std::fs::write(path, html)` で書き込み。失敗時は `state.error_message` にセット（既存の `GitError` 表示パターンを踏襲）。
- デフォルトファイル名は `diff-{short_id}.html`（2コミット比較時は `diff-{base_short}-{target_short}.html`）。

### 5. `src/ui/diff_panel.rs`（拡張）

**責務**:
- `show_file_list()` の直前（ファイル一覧の上部）に「HTMLエクスポート」ボタンを追加。
- `state.diff_files` が空でない場合のみ活性化。クリックで `state.needs_export = true` をセットするのみ（実際のエクスポート処理は行わない。UIレイヤーはGit/ファイルI/Oに関与しない既存ルールに従う）。

## データフロー

### エクスポート実行
```
1. ユーザーが差分パネルの「HTMLエクスポート」ボタンをクリック
2. diff_panel.rs: state.needs_export = true
3. app.rs update(): needs_export を検知 → export_diff_html() 呼び出し
4. export_diff_html(): 選択中コミット範囲の全DiffFileに対しhunksをrepo経由で取得
5. export::build_export_html() で自己完結HTML文字列を生成
6. rfd::FileDialog で保存先をユーザーに選択させる
7. std::fs::write() でファイルに書き込み。エラーは state.error_message に反映
```

## エラーハンドリング戦略

- Git読み込みエラー（`GitError`）は既存パターン通り `state.error_message` に文字列化してセットし、既存のエラーダイアログで表示する。
- ファイル書き込みエラー（`std::io::Error`）も同様に `state.error_message` に人間可読な日本語メッセージに変換してセットする。
- 保存ダイアログでユーザーがキャンセルした場合（`rfd` が `None` を返す）はエラー扱いにせず、何もせず終了する。

## テスト戦略

### ユニットテスト
- `src/export/html.rs`: `build_export_html()` の出力に対する文字列検証（エスケープ、行数表示、バイナリ注記、一覧⇔差分divのidの整合性）
- `src/git/diff.rs`: 移設後の `build_side_by_side_rows()` が既存テスト（`diff_panel.rs` 内にあれば移設）と同じ結果になること
- `count_changed_lines()` の集計が正しいこと

### 統合テスト
- 実際の一時Gitリポジトリ（`tempdir` パターン踏襲）で複数ファイル・複数コミットのシナリオを作り、`load_diff_files_between` → `build_export_html` の一連が動作し、生成HTMLに全ファイルパスが含まれることを確認

### 手動テスト（E2E）
- このリポジトリ自身を開き、単一コミット/2コミット比較それぞれでエクスポートボタンを押し、保存したHTMLをブラウザ（オフライン状態）で開いて一覧・差分表示・クリック切り替えを確認

## 依存ライブラリ

```toml
[dependencies]
rfd = "0.15"
```

選定理由: Windowsのネイティブ「名前を付けて保存」ダイアログを、追加のネイティブ依存なしに呼び出せる定番クレート。GUIフレームワーク非依存で eframe/egui と衝突しない。

## ディレクトリ構造

```
src/
├── export/              (新規)
│   ├── mod.rs
│   └── html.rs
├── git/
│   └── diff.rs           (SideCell/build_side_by_side_rows を移設、count_changed_lines追加)
├── ui/
│   └── diff_panel.rs      (エクスポートボタン追加、side-by-side整形ロジックの参照元を変更)
└── app.rs                 (AppState.needs_export、export_diff_html() 追加)
```

## 実装の順序

1. `src/git/diff.rs` に `SideCell`/`build_side_by_side_rows`/`count_changed_lines` を集約（`diff_panel.rs` から移設）
2. `Cargo.toml` に `rfd` を追加
3. `src/export/html.rs`（`ExportEntry`/`build_export_html`/`escape_html`）を実装 + ユニットテスト
4. `src/export/mod.rs` を追加し `main.rs` に `mod export;` を追加
5. `AppState.needs_export` 追加、`diff_panel.rs` にエクスポートボタン追加
6. `app.rs` に `export_diff_html()` 実装、`update()` ループに配線
7. 品質チェック（test/clippy/fmt/build）

## セキュリティ考慮事項

- 生成HTMLに埋め込むファイルパス・diff本文は必ず `escape_html()` を通す（XSS対策）。属性・テキストいずれも例外なし。
- 保存先パスはユーザーが `rfd` ダイアログで直接選択するため、パスインジェクションの余地はない（アプリ側でパス文字列を組み立てて外部入力から決定することはしない）。
- ネットワークアクセスは発生しない（完全ローカル処理）。

## パフォーマンス考慮事項

- 大量ファイル・大きなdiffの場合、全ファイルのhunksを事前に読み込むため一時的にメモリ使用量が増える。既存の「1MB超のファイルはdiff表示をスキップ」方針（`docs/architecture.md`）をエクスポートにも適用し、対象外ファイルは一覧に出すが差分本体は含めない。

## 将来の拡張性

- 出力形式の追加（Markdown等）は `src/export/` に新規モジュールを追加するだけで対応可能（`build_export_html` と同様のシグネチャの関数を追加）。
