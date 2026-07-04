# 設計書

## アーキテクチャ概要

既存の「CLI引数/Explorer右クリック起動」パスと同じ解決ロジック(`cli::resolve_target`)を、
OSファイルドロップの入力経路からも呼び出せるようにする。新しいレイヤーは作らず、
既存の `AppState` の状態遷移(`path_input` / `file_filter` / `needs_load` / `error_message`)に
合流させることで、履歴読み込み(`load_repo` → `load_commits_for_state`)は無改修のまま流用する。

```
[Explorer からドラッグ] → egui/eframe(winit) が RawInput.dropped_files にファイルパスを格納
        ↓
App::update() 冒頭で ctx.input(|i| i.raw.dropped_files.clone()) を取得
        ↓
first_dropped_path(&files) -- 純粋関数: 先頭の実パスを1件だけ抽出
        ↓
App::apply_dropped_path(path) -- cli::resolve_target(&path) を呼ぶ
        ├─ Ok(target) → state.path_input = repo_root文字列 / state.file_filter = target.file_filter
        │               state.needs_load = true / state.error_message = None
        └─ Err(e)      → state.error_message = Some(e.to_string())  (既存状態は変更しない)
        ↓
既存の needs_load 消化処理 → load_repo() → load_commits_for_state() → GitRepository::load_commits[_for_path]
```

## コンポーネント設計

### 1. `first_dropped_path`（新規・純粋関数、`src/app.rs` に追加）

**責務**:
- `&[egui::DroppedFile]` から、実際のファイルシステムパスを持つ先頭の1件を返す
- 複数ドロップされても2件目以降は無視する（合意済み仕様）

**実装の要点**:
- `egui::DroppedFile.path: Option<PathBuf>` が `None` のもの(ブラウザ限定のバイト列ドロップ等)はスキップして次の要素を見る
- eguiの型に依存するが、ロジック自体はGUIコンテキスト非依存なので単体テストしやすい

### 2. `App::apply_dropped_path`（新規メソッド、`src/app.rs` の `impl App` に追加）

**責務**:
- ドロップされた1パスを `cli::resolve_target` で解決し、`AppState` に反映する
- 成功時は現在のリポジトリ・履歴を（別リポジトリであっても）ドロップ先のものに切り替える
- 失敗時（Gitリポジトリ配下でない等）は既存の `error_message` に格納するのみで、現在の表示状態(リポジトリ・履歴・フィルタ)は変更しない

**実装の要点**:
- 既存の `AppState::new` 内の `cli_target` 解決ロジック（91-53行目付近の match）と同型の分岐を再利用する
- 「別リポジトリなら丸ごと切り替え」は、`state.path_input` と `state.file_filter` を書き換えて `needs_load = true` にするだけで実現できる（`load_repo()` が `path_input` から `GitRepository::open` し直すため、リポジトリの実体切り替えは既存コードパスがそのまま面倒を見る）

### 3. `App::update` への組み込み（`src/app.rs` の `impl eframe::App for App`）

**責務**:
- 毎フレーム、ドロップイベントの有無を確認し、あれば `apply_dropped_path` を呼ぶ

**実装の要点**:
- 既存の `needs_load` / `needs_diff_load` / `needs_file_load` チェックより前（`update()` 冒頭）に追加する
- `ctx.input(|i| i.raw.dropped_files.clone())` は毎フレーム呼んでもコストは軽微（空Vecがほとんど）。追加のポーリングフラグは不要
- ドラッグ中(`hovered_files`)のオーバーレイ演出はスコープ外につき実装しない

## データフロー

### フォルダをドラッグ&ドロップして履歴を絞り込む
```
1. ユーザーが Explorer でフォルダを選び、Gitwit ウィンドウ上にドロップする
2. winit/eframe がドロップイベントを RawInput.dropped_files に格納し、次フレームの update() に渡る
3. first_dropped_path が先頭の実パスを取り出す
4. apply_dropped_path が resolve_target を呼び、file_filter にフォルダの相対パスをセット、needs_load = true
5. 同じ update() フレーム中の needs_load 消化処理が load_repo → load_commits_for_state(file_filter あり) を実行
6. コミット一覧がフォルダ配下のみに絞り込まれ、トップバーに「履歴フィルタ: {path}」が表示される
```

### Gitリポジトリ外のパスをドロップした場合
```
1. resolve_target が Err(GitError::NotARepository(..)) を返す
2. apply_dropped_path が state.error_message にエラー文言をセットする（path_input/file_filter/commits は変更しない）
3. 既存のエラーモーダル（update() 内の error_message 表示ロジック）がそのまま表示する
4. ユーザーが「閉じる」を押すとエラーが消え、直前の表示状態に戻る
```

## エラーハンドリング戦略

### カスタムエラークラス

新規エラー型は不要。`cli::resolve_target` が返す既存の `GitError`（`src/git` で定義）をそのまま使う。

### エラーハンドリングパターン

`resolve_target` の `Err` はそのまま `to_string()` して `AppState.error_message` に格納し、既存のエラーモーダルに委譲する。新しいUI分岐は作らない。

## テスト戦略

### ユニットテスト
- `first_dropped_path`: 空配列 → `None`、`path: None` の要素のみ → `None`、複数要素の先頭が有効パス → 先頭を返す、先頭が `path: None` で2件目に有効パスがある場合の扱い（仕様上は「先頭1件のみ採用」なので `path` を持つ最初の要素を採用するか、単純に `files.first()` の `path` のみを見るかを実装時に確定し、テストで固定する）
- `apply_dropped_path` 相当のロジック: `resolve_target` 自体は `src/cli.rs` に既存テストがあるため重複させない。`App` 側は `resolve_target` の `Ok`/`Err` それぞれで `AppState` のどのフィールドが変わるかを検証する（`App`/`AppState` を直接構築してテスト可能な形にする）

### 統合テスト
- 本機能はOSのドラッグ&ドロップ実イベントに依存するため自動E2E化は行わない。手動確認（`docs/` の動作確認手順、または実行時の目視確認）でカバーする

## 依存ライブラリ

追加なし。`eframe`/`egui` の既存機能（`RawInput.dropped_files`）のみを使用する。

## ディレクトリ構造

新規ファイルなし。既存の `src/app.rs` に関数・メソッドを追加するのみ。

```
src/
  app.rs   ← first_dropped_path, App::apply_dropped_path を追加、update() に組み込み
  cli.rs   ← 変更なし(resolve_target を再利用)
```

## 実装の順序

1. `first_dropped_path` 純粋関数を `src/app.rs` に追加し、ユニットテストを書く
2. `App::apply_dropped_path` メソッドを追加し、`resolve_target` の Ok/Err 分岐で `AppState` を更新する
3. `eframe::App::update` の冒頭にドロップ検知呼び出しを組み込む
4. 手動動作確認（フォルダ/ファイル/別リポジトリ/リポジトリ外パス/複数ドロップの各シナリオ）
5. `cargo test` / `cargo clippy` / `cargo build` で品質確認

## セキュリティ考慮事項

- ドロップされたパスは `resolve_target` 内で `canonicalize()` されるため、シンボリックリンクや相対パス経由のトラバーサルは実パスに解決された上で `git2::Repository::discover` に渡る。既存のCLI引数経路と同じ検証を通るため新たな攻撃面は増えない
- ドロップされたパスをシェルコマンドに渡す処理は無い（git2のRustバインディングのみを使用）

## パフォーマンス考慮事項

- 毎フレームの `ctx.input(|i| i.raw.dropped_files.clone())` 呼び出しは、ドロップが無い限り空Vecのcloneでありコストは無視できる
- 履歴読み込み自体のパフォーマンス特性は既存の `load_commits_for_path` と同一（変更なし）

## 将来の拡張性

- 複数パスの同時ドロップを「マルチパスOR条件フィルタ」として扱う拡張は、`file_filter: Option<String>` を `Vec<String>` 化する設計変更が必要になるため、今回はスコープ外として明記した（requirements.md参照）
- ドラッグ中のハイライト演出を追加する場合は `ctx.input(|i| i.raw.hovered_files.clone())` を `update()` 内で読み、`CentralPanel` 描画時にオーバーレイを重ねる形で拡張できる
