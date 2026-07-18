# 設計書

> architecture-designer（バルベルデ）による設計検討済み。全文は
> `C:\Users\yata1\.claude\plans\lucky-puzzling-storm-agent-a3481661e3267f3d8.md` を参照。
> 本ファイルはその結論を実装用に整理したもの。

## アーキテクチャ概要

gitwit にとって初めてのネットワークI/O・バックグラウンドスレッド導入。既存の「UIはフラグを立てる →
`App::update()` が検知して処理する」パターン（`needs_load` 等）を踏襲しつつ、fetch だけは非同期化する。

```
[UI: toolbar]  「⟳ リモート取得」ボタン
      │  state.needs_fetch = true
      ▼
[App::update()]  needs_fetch 検知 → start_fetch(ctx)
      │  ・is_fetching ガード（多重起動防止）
      │  ・repo_path.clone() + ctx.clone() を用意
      │  ・std::thread::spawn( background fetch )
      ▼
[background thread]
      │  GitRepository::open(path)  ← スレッド内で新規に開き直す（git2::Repository は !Send）
      │  fetch_all_remotes() → list_remote_branches()
      │  tx.send(FetchOutcome{ result })
      │  ctx.request_repaint()       ← 忘れると結果が画面に反映されない
      ▼
[App::update()] 次フレーム冒頭で fetch_rx.try_recv()
      │  ・is_fetching = false
      │  ・Ok(names)  → state.remote_branches = names
      │  ・Err(e)     → state.error_message = Some(e.to_string())
      ▼
[UI: toolbar ComboBox] local_branches の下に区切り + remote_branches を追記表示
```

## コンポーネント設計

### 1. `src/git/remote.rs`（新規）

**責務**:
- 全リモートに対する `git fetch` の実行
- `refs/remotes/*` のリモートブランチ名一覧取得

**実装の要点**:
- `pub(super) fn fetch_all_remotes(repo: &Repository) -> Result<(), GitError>`
  - `repo.remotes()` で全リモート名を走査し、各リモートに対し `find_remote` → `fetch`
  - `git2::RemoteCallbacks::credentials()` で認証コールバックを設定:
    1. `allowed_types.contains(SSH_KEY)` なら `git2::Cred::ssh_key_from_agent(user)` を試す
    2. `allowed_types.contains(DEFAULT)` なら `git2::Cred::default()`（credential helper 経由。
       Windows 資格情報マネージャーを含む）
    3. `allowed_types.contains(USER_PASS_PLAINTEXT)` なら `git2::Cred::credential_helper(...)`
    4. 全て不可なら `Err` を返して打ち切る（**同一 cred を無条件に返すと libgit2 が無限ループする**
       ため、フォールバック順を必ず実装し、最後は確実に `Err` で終わらせる）
  - `FetchOptions::prune(git2::FetchPrune::On)` を設定し、リモート側で削除されたブランチの追従も行う
  - refspec は空配列（リモートの既定 refspec を使用）
- `pub(super) fn list_remote_branches(repo: &Repository) -> Result<Vec<String>, GitError>`
  - `repo.branches(Some(git2::BranchType::Remote))` を列挙
  - `"origin/HEAD"` のようなシンボリック参照（`name.ends_with("/HEAD")`）は除外
  - 名前昇順ソート（既存 `branch.rs::list_local_branches` と同じスタイル）
- **`egui::Context` やスレッド生成をこのファイルに持ち込まない**。同期の純関数として実装し、
  `commit.rs` / `branch.rs` と同じテスト容易性を保つ。

### 2. `GitRepository`（`src/git/repository.rs` 拡張）

**実装の要点**:
- `fetch_all_remotes(&self) -> Result<(), GitError>` / `list_remote_branches(&self) -> Result<Vec<String>, GitError>`
  を薄くラップして公開する（既存パターン踏襲）。

### 3. `src/git/mod.rs`

- `pub mod remote;` を追加。
- `GitError` の新規バリアントは不要。fetch 失敗は既存 `GitError::Git2` で表現し、`Display` 実装が
  そのまま日本語メッセージ化する。

### 4. `App` / `AppState`（`src/app.rs` 拡張）

**責務**:
- fetch のスレッド起動・結果ポーリング・`AppState` への反映を担う（スレッド生成と `egui::Context`
  はこのレイヤーの責務。git レイヤーへ UI 型を持ち込まない、CLAUDE.md のレイヤー分離原則に従う）。

**実装の要点**:
- `AppState` に追加:
  - `remote_branches: Vec<String>`（`"origin/main"` 形式）
  - `needs_fetch: bool`（UI がセット、`update()` が検知。既存フラグパターン踏襲）
  - `is_fetching: bool`（多重起動ガード兼 UI 上のボタン無効化・スピナー表示用）
  - `AppState::new()` と、テスト用ヘルパー `test_state()` の初期化子を両方更新する
- `App` 構造体に `fetch_rx: Option<std::sync::mpsc::Receiver<FetchOutcome>>` を追加
  （`Receiver` は `!Sync` だが `App` は UIスレッドでしか触らないため問題ない）
- `FetchOutcome` 型（`result: Result<Vec<String>, GitError>` を持つ）を `app.rs` 内に定義
- `App::update()` の**冒頭**（既存の `needs_*` 検知より前）に `fetch_rx` の `try_recv()` ポーリングを追加:
  - `Ok(outcome)` を受信したら `is_fetching = false`、`fetch_rx = None`、結果を
    `remote_branches` または `error_message` に反映
- 既存の `needs_*` フラグ検知ブロック群に `needs_fetch` の検知を追加し、`start_fetch(ctx)` を呼ぶ
- `fn start_fetch(&mut self, ctx: &egui::Context)`:
  - `is_fetching` が真なら即 return（多重起動ガード）
  - `state.repo_path` が `None` なら return
  - `mpsc::channel()` を生成して `self.fetch_rx` にセット、`is_fetching = true`
  - `ctx.clone()` と `path.clone()` を `move` して `std::thread::spawn`:
    - スレッド内で `GitRepository::open(&path)` → `fetch_all_remotes()` → `list_remote_branches()`
    - `tx.send(FetchOutcome { result })`（受信側が drop 済みでも `let _ =` でエラー無視。
      アプリ終了時にスレッドが detached のまま走っても実害はない）
    - 最後に `ctx.request_repaint()` を必ず呼ぶ

### 5. `ui/toolbar.rs`（拡張）

**実装の要点**:
- 「⟳ リモート取得」ボタンを追加。`ui.add_enabled_ui(!state.is_fetching, |ui| { ... })` で
  fetch 中は無効化し、`state.is_fetching` が真なら `ui.spinner()` を隣に表示。
- クリックで `state.needs_fetch = true` をセットするのみ（実際の fetch 起動は `App::update()` 側）。
- 既存 `show_branch_selector` の `ComboBox::show_ui` 内で、`local_branches` の一覧の後に
  `ui.separator()` を挟み、`remote_branches` を並べて表示する。リモートブランチの項目は
  クリックしても `pending_branch_switch` をセットしない（チェックアウト対象外、表示のみ）。

## データフロー

（上記アーキテクチャ概要の図を参照）

## エラーハンドリング戦略

- fetch 失敗（ネットワーク未接続・認証失敗等）は `GitError::Git2` としてそのまま
  `state.error_message` に反映し、既存のエラーダイアログ（`App::update()` 内の
  `egui::Window::new("エラー")`）で表示する。
- エラー表示後もボタンは再度クリック可能な状態に戻る（`is_fetching = false` を確実にセットする）。

## テスト戦略

### ユニットテスト（`src/git/remote.rs` 内、`#[cfg(test)] mod tests`）

- `fetch_all_remotes` / `list_remote_branches`: tempdir に2つのローカルリポジトリ（bare の
  "リモート"役と、それを origin として `clone` した"ローカル"役）を作り、`file://` プロトコルで
  fetch する。認証不要でロジックを検証できる（`branch.rs` の tempdir テストパターンを踏襲）。
- リモート側に新しいブランチを追加してから fetch し、`list_remote_branches` にそのブランチが
  `"origin/<name>"` 形式で現れることを確認する。
- `"origin/HEAD"` が一覧から除外されることを確認する。

### 統合テスト・UIテスト

- スレッド・チャネル配線（`start_fetch` の起動〜受信）は既存方針通り UI レイヤーとして
  手動確認とする（egui のテストは自動化困難なため）。実機でボタンを押し、fetch 中も他の操作が
  可能なこと、完了後に一覧が更新されることを確認する。

## 依存ライブラリ

新規ライブラリの追加なし。`std::thread` / `std::sync::mpsc`（標準ライブラリ）と、既存の
`git2 = "0.19"` の `Remote::fetch` / `RemoteCallbacks` / `FetchOptions` API のみで実装する。

## ディレクトリ構造

```
src/
  git/
    mod.rs        (変更: remote モジュール追加)
    remote.rs      (新規: fetch_all_remotes, list_remote_branches)
    repository.rs  (変更: GitRepository にラッパーメソッド追加)
  app.rs           (変更: AppState に remote_branches/needs_fetch/is_fetching、
                    App に fetch_rx フィールドと start_fetch メソッド追加)
  ui/
    toolbar.rs     (変更: 「リモート取得」ボタン + スピナー + ComboBox にリモートブランチ表示)
docs/
  architecture.md  (変更: ネットワーク接続制約の解除、「並行処理モデル」章の追加)
```

## 実装の順序

1. `src/git/remote.rs` の2純関数を実装 + ユニットテスト
2. `GitRepository` にラッパーメソッド追加（`repository.rs`）+ `mod.rs` に登録
3. `AppState` にフィールド追加、`App` に `fetch_rx` 追加、全初期化子（`new` / `test_state`）を更新
   → コンパイルが通る状態にする
4. `App::start_fetch` の実装 + `update()` へのポーリング・フラグ検知の配線
5. `toolbar.rs` にボタン・スピナー・ComboBox 表示を追加
6. 手動E2E確認（実リモートでの fetch、UIが固まらないこと、一覧表示、エラー時の非クラッシュ、
   連打時の多重起動防止）
7. `docs/architecture.md` 改訂
8. `cargo test` / `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` / `cargo build`

## セキュリティ考慮事項

- 認証はすべて SSH エージェント / OS の credential helper に委譲し、アプリ側でパスワード等の
  シークレットを一切保持・入力させない（`Cred::ssh_key_from_agent` / `Cred::default()` /
  `Cred::credential_helper` のみを使用）。
- 認証コールバックは必ず有限回で `Err` に到達するようにし、libgit2 の無限リトライループを防ぐ。
- fetch 対象は `repo.remotes()` が返す既存のリモート設定のみで、ユーザーが任意の URL を
  都度入力する経路はない。

## パフォーマンス考慮事項

- fetch はネットワークI/Oのため数秒かかりうるが、バックグラウンドスレッドで実行するため
  UIスレッド（60fps 前提）をブロックしない。
- `is_fetching` ガードにより、ボタン連打による多重 fetch（同時に複数スレッドが同一リポジトリへ
  アクセスする事態）を防ぐ。

## 将来の拡張性

- 同じ「`std::thread` + `mpsc` + `request_repaint`」パターンは、将来の push/pull 等の
  非同期操作にもそのまま適用できる。非同期操作が3つ以上に増えた場合は `poll-promise` クレート等の
  導入を再検討する余地がある（今回は fetch 1箇所のみのため見送り）。
- リモートブランチのチェックアウト（トラッキングブランチ作成）は、今回作った
  `list_remote_branches` の戻り値をそのまま使い、`branch.rs::checkout_branch` に類する関数を
  `remote.rs` へ追加する形で自然に拡張できる。
