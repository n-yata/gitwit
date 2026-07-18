# 設計書

## アーキテクチャ概要

既存のレイヤー構成（UI → AppState/App → Git ロジック）をそのまま踏襲する。
コミット履歴読み込み（`src/git/commit.rs` → `GitRepository` → `App::load_repo`）と同じパターンで、
新規モジュール `src/git/branch.rs` にブランチ一覧・現在ブランチ取得・checkout の純粋なロジックを実装し、
`GitRepository` に薄いラッパーメソッドを追加、`App`/`AppState` で状態を保持、`toolbar.rs` に UI を追加する。

```
ui/toolbar.rs (ComboBox)
  → AppState.pending_branch_switch フラグ
    → App::update() が検知
      → App::switch_branch()
        → GitRepository::checkout_branch() → git/branch.rs::checkout_branch()
          → 成功: App::load_repo() 相当の再読込 (commits, current_branch, local_branches)
          → 失敗: AppState.error_message にセット（ブランチは切り替わらない）
```

## コンポーネント設計

### 1. `src/git/branch.rs`（新規）

**責務**:
- ローカルブランチ名一覧の取得（`repo.branches(Some(BranchType::Local))`）
- 現在の HEAD が指すブランチ名の取得（detached の場合は `None`）
- 指定ブランチへの checkout（作業ツリー変更との衝突時はエラーを返す）

**実装の要点**:
- `list_local_branches` はブランチ名を昇順ソートして返す（一覧の表示順を安定させるため）。
- `current_branch_name` は `repo.head()` が `UnbornBranch`（空リポジトリ）の場合、または
  `head.is_branch()` が `false`（detached HEAD）の場合に `Ok(None)` を返す。
- `checkout_branch` は `git2::build::CheckoutBuilder::safe()`（デフォルト、force ではない）を使い、
  作業ツリーとの衝突がある場合は `git2::ErrorCode::Conflict` を検知して `GitError::CheckoutConflict` に変換する。
  checkout 成功後に `repo.set_head("refs/heads/<name>")` で HEAD を更新する。
- 既存コミット読み込み（`commit.rs`）と同じく `pub(super)` 関数として実装し、`GitRepository` 経由でのみ公開する。

### 2. `GitRepository`（`src/git/repository.rs` 拡張）

**責務**:
- `branch.rs` の関数を薄くラップして公開 API として提供する（既存の `load_commits` 等と同じパターン）。

**実装の要点**:
- `list_local_branches(&self) -> Result<Vec<String>, GitError>`
- `current_branch_name(&self) -> Result<Option<String>, GitError>`
- `checkout_branch(&self, name: &str) -> Result<(), GitError>`

### 3. `GitError`（`src/git/mod.rs` 拡張）

**責務**:
- checkout 衝突を表す新しいバリアント `CheckoutConflict` を追加し、ユーザー向けメッセージに変換する。

**実装の要点**:
- `GitError::CheckoutConflict` → 「作業ツリーに未コミットの変更があるため、ブランチを切り替えられません」
- 既存の `Display` / `source()` の match を網羅的に更新する（コンパイルエラーで漏れを検出できる）。

### 4. `AppState` / `App`（`src/app.rs` 拡張）

**責務**:
- 現在のブランチ名・ローカルブランチ一覧を状態として保持する。
- ブランチ切り替え要求（UI からのフラグ）を検知し、`GitRepository::checkout_branch` を呼んで結果を反映する。

**実装の要点**:
- `AppState` に以下を追加:
  - `current_branch: Option<String>` — `None` は detached HEAD または空リポジトリ
  - `local_branches: Vec<String>`
  - `pending_branch_switch: Option<String>` — UI がセットする「切り替えたいブランチ名」
- `load_repo()` 内で commits 読み込みに続けて `current_branch` / `local_branches` も読み込む
  （小さな private helper `load_branch_state(&mut self, repo: &GitRepository)` を追加して `load_repo` から呼ぶ）。
- `App::update()` の既存の `needs_*` フラグ処理と同じ並びで、
  `pending_branch_switch.take()` を検知したら `switch_branch(name)` を呼ぶ。
- `switch_branch(&mut self, name: String)`:
  - `checkout_branch` 成功時: `load_repo()` と同様にコミット一覧・ブランチ状態を再読込し、
    `selected_commits` / `diff_files` / `selected_file` / `diff_hunks` をクリアする（受け入れ条件: diff 選択のクリア）。
  - 失敗時: `error_message` にセットし、他の状態（現在のブランチ・コミット一覧）は変更しない。

### 5. `ui/toolbar.rs`（拡張）

**責務**:
- 現在のブランチ名の表示と、`egui::ComboBox` によるローカルブランチ選択 UI。

**実装の要点**:
- リポジトリが開かれている（`state.repo_path.is_some()`）場合のみブランチセレクタを表示する。
- 表示ラベルは `state.current_branch` があればその名前、`None` なら「(detached HEAD)」。
- `ComboBox::from_id_salt("branch_selector")` の `show_ui` 内で `state.local_branches` をループし、
  選択中と異なるブランチがクリックされたら `state.pending_branch_switch = Some(branch_name)` をセットするのみ
  （実際の checkout は `App::update()` 側で行う。既存の `needs_load` と同じ「UI はフラグを立てるだけ」の方針を踏襲）。

## データフロー

### ブランチ切り替え（正常系）
```
1. ユーザーがツールバーのブランチ名をクリックし、ComboBox からブランチBを選択
2. toolbar.rs が state.pending_branch_switch = Some("B") をセット
3. App::update() が pending_branch_switch を検知し switch_branch("B") を呼ぶ
4. GitRepository::checkout_branch("B") が成功
5. コミット一覧・current_branch・local_branches を再読込
6. selected_commits / diff_files / selected_file / diff_hunks をクリア
7. UI が新しいブランチのコミット一覧を表示
```

### ブランチ切り替え（未コミット変更の衝突）
```
1. ユーザーがブランチBを選択
2. checkout_branch が git2::ErrorCode::Conflict を検知 → GitError::CheckoutConflict
3. App が error_message にセット。current_branch・commits は変更しない
4. 既存のエラーダイアログ（App::update 内の egui::Window::new("エラー")）がそのまま表示
```

## エラーハンドリング戦略

### カスタムエラークラス

`GitError` に `CheckoutConflict` バリアントを追加（データは持たず固定メッセージ。将来的に
衝突ファイル一覧を表示する拡張の余地は残すが、今回のスコープでは固定文言のみ）。

### エラーハンドリングパターン

既存パターンを踏襲: `git/branch.rs` は `Result<T, GitError>` を返し、`App` 側で
`match` して `AppState.error_message` に `to_string()` した文字列をセットする。`unwrap`/`expect` は使わない。

## テスト戦略

### ユニットテスト（`src/git/branch.rs` 内、`#[cfg(test)] mod tests`）

`commit.rs` の既存テストと同じパターンで、一時ディレクトリに `git2::Repository::init` して検証する。

- `list_local_branches`:
  - 複数ブランチを作成した場合、名前でソートされた一覧が返る
  - ブランチが1つ（現在のブランチ）しかない場合もそれが返る
- `current_branch_name`:
  - 通常のブランチ上では `Some("ブランチ名")` を返す
  - detached HEAD（`repo.set_head_detached`）の場合は `None` を返す
- `checkout_branch`:
  - 別ブランチへの checkout 成功後、`repo.head()` の shorthand が切り替え先になっている
  - 作業ツリーに切り替え先と衝突する未コミット変更がある場合、`GitError::CheckoutConflict` を返し、
    実際の HEAD は元のブランチのままである

### 統合テスト

今回はロジックが `src/git/branch.rs` のユニットテストで十分にカバーできるため、
`tests/git_integration.rs` への追加は行わない（既存方針: UI レイヤーは手動確認）。

## 依存ライブラリ

新規ライブラリの追加なし。既存の `git2 = "0.19"` の API のみで実装可能
（`Repository::branches`, `Repository::head`, `Repository::checkout_tree`, `Repository::set_head`）。

## ディレクトリ構造

```
src/
  git/
    mod.rs        (変更: branch モジュール追加、GitError に CheckoutConflict 追加)
    branch.rs      (新規: list_local_branches, current_branch_name, checkout_branch)
    repository.rs  (変更: GitRepository にラッパーメソッド追加)
  app.rs           (変更: AppState に current_branch/local_branches/pending_branch_switch、App に switch_branch)
  ui/
    toolbar.rs     (変更: ブランチ名表示 + ComboBox)
```

## 実装の順序

1. `GitError::CheckoutConflict` バリアント追加（`src/git/mod.rs`）
2. `src/git/branch.rs` 実装（list_local_branches, current_branch_name, checkout_branch）+ ユニットテスト
3. `GitRepository` にラッパーメソッド追加（`src/git/repository.rs`）
4. `AppState` にフィールド追加、`App::load_repo`/`switch_branch`/`update` を拡張（`src/app.rs`）
5. `toolbar.rs` にブランチ表示・ComboBox を追加
6. 手動確認（`cargo run` でリポジトリを開き、実際にブランチ切り替え・detached HEAD・未コミット変更時の衝突を確認）
7. `cargo test` / `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` / `cargo build`

## セキュリティ考慮事項

- checkout 先のブランチ名は `ComboBox` から選択されたものに限定され、`repo.branches()` が返した
  既存のローカルブランチ名以外は選択できない。ユーザーが任意文字列を打ち込んでコマンドを組み立てる
  経路は存在しない（テキスト入力欄を使わない設計）ため、パスインジェクション等のリスクはない。
- `git2::build::CheckoutBuilder` はデフォルト（`safe`、force なし）を使用し、意図せず作業ツリーの
  変更を破棄しない。

## パフォーマンス考慮事項

- `list_local_branches` はローカルブランチのみを走査するため、既存の `collect_refs`（全 references 走査）
  より軽量。リポジトリを開くたび・切り替えるたびに毎回呼んでも問題ない規模と想定する。

## 将来の拡張性

- 今回スコープ外としたリモートブランチ一覧・force checkout・stash・ブランチ作成/削除/マージは、
  `src/git/branch.rs` に関数を追加する形で自然に拡張できる（`GitError::CheckoutConflict` に
  衝突ファイル一覧を持たせる拡張も含め、モジュールを分けたことで影響範囲を局所化できる）。
