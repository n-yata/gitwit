# 振り返り: コミット履歴表示

**実装完了日**: 2026-07-02

## 計画と実績の差分

| 項目 | 計画 | 実績 |
|------|------|------|
| ファイル数 | 9ファイル | 9ファイル（計画通り） |
| ビルド | 1発通過を目標 | コンパイルエラー4件を修正が必要だった |
| テスト | 5件 | 5件（全通過） |
| 追加修正 | なし | implementation-validator 指摘で4件追加修正 |

## ハマりどころ

### 0. egui のデフォルトフォントに日本語グリフが含まれない
egui はデフォルトで英数字のみのフォントを使用する。日本語テキストが □ で表示される。
`CreationContext::egui_ctx` に Windows システムフォント（Meiryo 等）を `FontDefinitions` として渡すことで解決。

**次回注意点**: egui で日本語を使う場合、`App::new()` 内で必ず `setup_japanese_font()` を呼ぶ。
候補フォントは `meiryo.ttc` → `YuGothR.ttc` → `msgothic.ttc` の順に fallback する。

### 1. `egui::Margin::symmetric` の型が `i8`
egui 0.31.x では `Margin::symmetric(x, y)` の引数が `i8`（整数）。
`f32` を渡すとコンパイルエラー。

**次回注意点**: egui のバージョンを固定したまま使うなら、マージンには整数を使う。

### 2. `ErrorCode::NotARepository` は git2 0.19 に存在しない
git2 の新しいバージョンでは存在するが、0.19 では `ErrorClass::Repository` で判定する必要がある。

**次回注意点**: git2 の API は バージョンによって異なる。エラーコードを使うときは docs.rs で現行バージョンを確認する。

### 3. egui ウィンドウの二重可変借用
`Window::open(&mut open)` + クロージャ内で `open = false` の組み合わせは Rust の借用チェッカーに弾かれる。
`close_error` フラグを別変数に分離して、クロージャの外で状態を更新するパターンで解決。

**次回注意点**: egui の `Window::open()` を使う場合、close フラグはクロージャの外で処理する。

## 学んだこと

### implementation-validator が有用だった指摘
- `collect_refs` の O(n×m) → HashMap で O(n) に改善。自分では気づかなかったアルゴリズム改善
- `load_commits` の可視性（`pub` → `pub(super)`）。設計意図を実装に反映させる意識の重要性
- `format_relative_time` の re-export。内部モジュールを UI から直接参照しない原則

### `#[allow(dead_code)]` の使い所
次フェーズで使う予定のフィールドには `#[allow(dead_code)]` + コメントを付ける慣例を決めた。

## 次回への申し送り

### diff 表示フェーズ（次の機能）で必要なこと
- `CommitInfo.oid` を使って `GitRepository::load_diff_files(oid)` を実装する
- `src/git/diff.rs` を新規作成（`DiffFile`, `DiffHunk`, `DiffLine` の定義と取得）
- `src/ui/diff_panel.rs` を新規作成（右ペイン: ファイル一覧 + diff 表示）
- `AppState` に `diff_files`, `selected_file`, `diff_hunks` フィールドを追加
- `app.rs` のレイアウトを左右分割に変更（現在は中央パネルのみ）

### テスト追加の優先度（implementation-validator 指摘より）
- `AppConfig` の TOML 往復テスト（`save_config` → `load_config` の整合性）
- `format_relative_time` の境界値テスト（`diff = 60` ちょうど、`diff < 0`）
- `GitRepository::open` に存在しないパスを渡した場合の `GitError::NotARepository` 確認
