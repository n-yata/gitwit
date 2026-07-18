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

## フェーズ1: Git ロジック層（src/git/）

- [x] `GitError` に `CheckoutConflict` バリアントを追加（`src/git/mod.rs`）
  - [x] `Display` の match 分岐にメッセージ「作業ツリーに未コミットの変更があるため、ブランチを切り替えられません」を追加
  - [x] `source()` の match 分岐も網羅的に更新
- [x] `src/git/branch.rs` を新規作成
  - [x] `list_local_branches(repo: &Repository) -> Result<Vec<String>, GitError>` を実装（名前昇順ソート）
  - [x] `current_branch_name(repo: &Repository) -> Result<Option<String>, GitError>` を実装（detached/unborn は `None`）
  - [x] `checkout_branch(repo: &Repository, branch_name: &str) -> Result<(), GitError>` を実装（`CheckoutBuilder::safe` + `set_head`、衝突時 `CheckoutConflict`）
  - [x] ユニットテスト: `list_local_branches` が複数ブランチを名前順で返す
  - [x] ユニットテスト: `current_branch_name` が通常ブランチで `Some(名前)` を返す
  - [x] ユニットテスト: `current_branch_name` が detached HEAD で `None` を返す
  - [x] ユニットテスト: `checkout_branch` が別ブランチへの切り替えに成功し `repo.head()` が切り替わる
  - [x] ユニットテスト: `checkout_branch` が未コミット変更との衝突時に `CheckoutConflict` を返し、HEAD が変わらない
- [x] `src/git/mod.rs` に `pub mod branch;` を追加

## フェーズ2: リポジトリラッパー層

- [x] `GitRepository`（`src/git/repository.rs`）にラッパーメソッドを追加
  - [x] `list_local_branches(&self) -> Result<Vec<String>, GitError>`
  - [x] `current_branch_name(&self) -> Result<Option<String>, GitError>`
  - [x] `checkout_branch(&self, name: &str) -> Result<(), GitError>`

## フェーズ3: アプリ状態・イベントループ（src/app.rs）

- [x] `AppState` にフィールドを追加
  - [x] `current_branch: Option<String>`
  - [x] `local_branches: Vec<String>`
  - [x] `pending_branch_switch: Option<String>`
  - [x] `AppState::new()` の初期化を更新
  - [x] テスト用ヘルパー `test_state()` の初期化も更新
- [x] `App::load_repo()` を拡張し、コミット読み込みに続けて `current_branch` / `local_branches` を読み込む
  - [x] private helper `load_branch_state(&mut self, repo: &GitRepository)` を追加して呼び出す
- [x] `App::switch_branch(&mut self, name: String)` を実装
  - [x] 成功時: コミット一覧・ブランチ状態を再読込し、`selected_commits`/`diff_files`/`selected_file`/`diff_hunks` をクリア
  - [x] 失敗時: `error_message` にセットし、他の状態は変更しない
- [x] `App::update()` に `pending_branch_switch.take()` の検知処理を追加し、`switch_branch` を呼ぶ

## フェーズ4: UI（src/ui/toolbar.rs）

- [x] リポジトリが開かれている場合に現在のブランチ名（または「(detached HEAD)」）を表示
- [x] `egui::ComboBox` でローカルブランチ一覧を表示し、選択時に `state.pending_branch_switch` をセット

## フェーズ5: 動作確認

- [x] `cargo run` で実リポジトリ（本プロジェクト自身の worktree）を開き、以下を手動確認する（スクリーンショットで確認済み）
  - [x] ツールバーに現在のブランチ名が表示される（`feature/branch-switch` と表示）
  - [x] ドロップダウンからブランチを切り替えると、コミット一覧が新ブランチの履歴に更新される（`feature/add-mit-license` へ切替後、履歴が切り替わることを確認）
  - [x] 切り替え後、diff パネルの選択がクリアされている（「コミットを選択してください」表示に戻る）
  - [x] リモートブランチ（`origin/xxx`）が一覧に出ない（`feature/add-mit-license`/`feature/branch-switch`/`master` のみ表示）
  - [x] checkout が失敗するケースでエラーダイアログが出て、ブランチが変わらない
    （未コミット変更との衝突は `src/git/branch.rs` のユニットテストで直接検証済み。手動確認では、
    別 worktree で使用中の `master` への切り替えを試み、git2 のエラー
    `cannot set HEAD to reference 'refs/heads/master' as it is the current HEAD of a linked repository`
    がエラーダイアログに表示され、アプリはクラッシュせず、ブランチ表示も `feature/branch-switch` のまま
    変わらないことを実際に確認した）

## フェーズ6: 品質チェックと修正

- [x] すべてのテストが通ることを確認
  - [x] `cargo test`（44 件全て成功、うち今回追加した5件を含む）
- [x] リント・フォーマットに問題がないことを確認
  - [x] `cargo clippy --all-targets -- -D warnings`（警告ゼロ）
  - [x] `cargo fmt --check`（今回変更したファイル `src/git/branch.rs` `src/git/repository.rs`
    `src/git/mod.rs` `src/app.rs` `src/ui/toolbar.rs` に差分なし。`src/cli.rs` `src/git/commit.rs` に
    既存の差分が出るが、この branch では未変更のファイルであり、ローカル rustfmt バージョン差による
    既存ドリフトと確認済み。詳細は retrospective.md に記録）
- [x] ビルドが成功することを確認
  - [x] `cargo build`

## フェーズ7: ドキュメント更新

- [x] README.md を更新（必要に応じて）（README は機能を列挙する形式ではなく更新不要と判断。理由: 冒頭の説明文は「コミット履歴一覧・diff確認」という既存MVPの要約タグラインで、機能網羅リストではないため）
- [x] 実装後の振り返りを記録（別ファイル `retrospective.md` に記録 → モード3）

---

> **振り返りについて**: 実装後の振り返りはこのファイルではなく、同じディレクトリの
> `retrospective.md` に記録する（テンプレート: `.claude/skills/steering/templates/retrospective.md`）。
> 全タスクが `[x]` になったことを確認してから作成すること。
