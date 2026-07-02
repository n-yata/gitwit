# 振り返り: diff表示

**実装完了日**: 2026-07-02

## 計画と実績の差分

| 項目 | 計画 | 実績 |
|------|------|------|
| ファイル数 | 8ファイル変更/新規 | 8ファイル（計画通り） |
| ビルド | 1発通過 | 警告1件修正が必要だった（DiffLine 未使用 re-export） |
| テスト | 既存5件全通過 | 5件全通過（新規テストは追加できず） |
| Clippy | 0エラー | 2件修正が必要だった（clamp + len_zero） |
| バリデーター指摘 | 0件想定 | 3件指摘（全修正済み） |

## ハマりどころ

### 0. `git2::Deltas::is_empty()` が unstable
`clippy::len_zero` の指摘に従い `len() == 0` を `.is_empty()` に変更したところ、コンパイルエラー。
`git2::Deltas` は `ExactSizeIterator` を実装しており、その `is_empty()` は Rust stable では未安定（`exact_size_is_empty` feature gate が必要）。

**解決策**: `len() == 0` に戻して `#[allow(clippy::len_zero)]` を付与。コメントに理由を記載。

**次回注意点**: `ExactSizeIterator` を実装するカスタム型（git2 等のライブラリ型）に対して clippy が `.is_empty()` を推奨しても、実際にコンパイルできない場合がある。試してみてエラーが出たら `len() == 0` + allow で対処する。

### 1. `foreach` クロージャの二重可変借用
`git2::Diff::foreach()` の `hunk_cb` と `line_cb` の両方から `hunks: Vec<DiffHunk>` を可変借用する必要があり、Rust の借用チェッカーに弾かれる。

**解決策**: `git2::Patch` API を使用。`Patch::from_diff(&diff, idx)` → `patch.num_hunks()` → `patch.hunk(i)` → `patch.line_in_hunk(hunk_i, line_i)` と全て同期的なメソッド呼び出しで取得できる。

**次回注意点**: `git2::Diff::foreach()` は複数のクロージャが同じデータを可変参照する必要がある場合に使えない。`Patch` API を優先的に選ぶ。

### 2. `DiffLine` re-export の方針
`DiffLine` を `mod.rs` で re-export するとコンパイラが "unused import" 警告を出す（UI が型名を直接使用しないため）。
バリデーターは API の一貫性として追加を推奨したが、コンパイラ警告を優先して非公開のままにした。

**判断の根拠**: `DiffLine` は `DiffHunk::lines` の要素型として暗黙的に公開されており、外部から名前で参照する必要がない。Rust の「使われない re-export は警告」というポリシーに従い削除状態を維持する。

### 3. `update()` 末尾の重複 `needs_load` ブロック
元の `app.rs` にあった「末尾の `needs_load` チェック」が今回の新規実装に引き継がれた。
バリデーターが「ツールバー内でフラグが再セットされた場合に描画中に状態変化するリスク」を指摘。削除して修正した。

**次回注意点**: `update()` 内でのロード呼び出しは最上部の1箇所のみ。描画コード内でのフラグセットは避け、フラグのセットは UI 関数の戻り値や状態変更で行う。

## 学んだこと

### git2 の Patch API が foreach より遥かに使いやすい
`Diff::foreach()` はコールバック地獄になりがち。`Patch::from_diff()` は同期的なメソッド呼び出しで済むため、Rust の借用チェッカーと相性が良い。
diff 取得は `Patch` API を使うのが標準パターン。

### バリデーターが有用だった指摘
- `update()` 末尾の重複 `needs_load` ブロック: コードを引き継いだときに見落としやすいバグの温床
- `file_path` と `is_binary` の二重 `get()` の一本化: ボロー回数削減

### 4. egui SidePanel はリサイズ不能（コンテンツ幅が上書きされる）

`SidePanel::left().resizable(true)` を使ってもドラッグでサイズが変わらない問題が発生。

**根本原因**: egui の `SidePanel` はフレーム毎に `panel_ui.min_rect().width()` をパネル幅として記録する。ScrollArea がパネル全体を埋めると `min_rect` がパネル幅と一致するため、ドラッグで変更した幅が即座に上書きされてしまう。

**試みて失敗したこと**:
- `set_min_width` 削除
- `auto_shrink([false, false])` 削除
- ラベルに `truncate()` 追加
- `.resizable(true)` 明示

**最終的な解決策**: `SidePanel` を完全廃止し、`CentralPanel` + 手動 `allocate_new_ui` レイアウトに置き換え。`AppState.split_x: f32` でセパレータ位置を保持し、`ui.interact(sep_rect, id, Sense::drag())` でドラッグを検出。

**次回注意点**: egui でリサイズ可能な水平分割レイアウトが必要な場合は、最初から手動レイアウト（CentralPanel + allocate_new_ui）を選ぶ。SidePanel のリサイズは ScrollArea との組み合わせで動作しない。

### 5. `allocate_new_ui` はクリップ矩形を自動設定しない

`ui.allocate_new_ui(UiBuilder::new().max_rect(rect), |ui| { ... })` はコンテンツをサブ UI の `max_rect` に制限するが、描画のクリッピングは行わない。そのため、コンテンツが隣のパネルにはみ出して描画される問題が発生。

**解決策**: サブ UI クロージャ内で明示的に `ui.set_clip_rect(rect.intersect(ui.clip_rect()))` を呼ぶ。`ui.clip_rect()` との intersect を取ることで親の clip 範囲を超えないようにする。

**次回注意点**: `allocate_new_ui` を使う場合は必ずクリップ矩形を手動設定すること。

## 次回への申し送り

### 追加できなかったテスト
- `git2` のテスト用リポジトリ（`Repository::init` + `Signature` 等）を使った `load_diff_files` / `load_diff_hunks` の単体テスト
- `AppState` の状態遷移テスト（コミット選択時にクリアが正しく行われるか）
- 前フェーズ（コミット履歴表示）からの申し送り: `AppConfig` のTOML往復テスト、`GitRepository::open` のエラーケーステスト

### Post-MVP で必要な実装
- ブランチグラフ可視化（コミット一覧左にグラフライン）
- コミット・ステージング（write 系 git2 操作）
- プッシュ・プル（ネットワーク操作＝認証が必要）
- ブランチの作成・切り替え・マージ
- 1MB超ファイルのサイズチェック（diff 取得前に blob size を確認）
- シンタックスハイライト（tree-sitter 等の利用を検討）
