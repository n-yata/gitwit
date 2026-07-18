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

## フェーズ1: Git ロジック層（src/git/remote.rs）

- [x] `src/git/remote.rs` を新規作成
  - [x] `fetch_all_remotes(repo: &Repository) -> Result<(), GitError>` を実装
    - [x] `repo.remotes()` で全リモートを走査し `find_remote` → `fetch`
    - [x] `RemoteCallbacks::credentials()` に ssh-agent → `Cred::default()` → credential_helper の
          フォールバック順を実装し、全滅時は `Err` で打ち切る
    - [x] `FetchOptions::prune(FetchPrune::On)` を設定
  - [x] `list_remote_branches(repo: &Repository) -> Result<Vec<String>, GitError>` を実装
    - [x] `BranchType::Remote` で列挙し、`"/HEAD"` で終わる名前を除外
    - [x] 名前昇順ソート
  - [x] ユニットテスト: tempdir に bare リポジトリ（リモート役）+ それを clone したリポジトリ
        （ローカル役）を作り、`file://` で fetch → `list_remote_branches` に反映されることを確認
  - [x] ユニットテスト: リモート側に新しいブランチを追加後 fetch すると、そのブランチが一覧に現れる
  - [x] ユニットテスト: `origin/HEAD` が一覧から除外されることを確認
- [x] `src/git/mod.rs` に `pub mod remote;` を追加

## フェーズ2: リポジトリラッパー層

- [x] `GitRepository`（`src/git/repository.rs`）にラッパーメソッドを追加
  - [x] `fetch_all_remotes(&self) -> Result<(), GitError>`
  - [x] `list_remote_branches(&self) -> Result<Vec<String>, GitError>`

## フェーズ3: アプリ状態・非同期配線（src/app.rs）

- [x] `AppState` にフィールドを追加
  - [x] `remote_branches: Vec<String>`
  - [x] `needs_fetch: bool`
  - [x] `is_fetching: bool`
  - [x] `AppState::new()` の初期化を更新
  - [x] テスト用ヘルパー `test_state()` の初期化も更新
- [x] `FetchOutcome` 型（`result: Result<Vec<String>, GitError>`）を定義
- [x] `App` 構造体に `fetch_rx: Option<std::sync::mpsc::Receiver<FetchOutcome>>` を追加
  （`App::new()` の初期化も更新）
- [x] `App::update()` の冒頭に `fetch_rx` の `try_recv()` ポーリングを追加
  - [x] 受信できたら `is_fetching = false`、`fetch_rx = None`
  - [x] `Ok(names)` → `state.remote_branches = names`
  - [x] `Err(e)` → `state.error_message = Some(e.to_string())`
- [x] `App::update()` の既存 `needs_*` 検知ブロック群に `needs_fetch` の検知を追加し `start_fetch(ctx)` を呼ぶ
- [x] `App::start_fetch(&mut self, ctx: &egui::Context)` を実装
  - [x] `is_fetching` が真なら早期 return（多重起動ガード）
  - [x] `repo_path` が `None` なら早期 return
  - [x] `mpsc::channel()` 生成、`fetch_rx` にセット、`is_fetching = true`
  - [x] `ctx.clone()` + `path.clone()` を move した `std::thread::spawn` でバックグラウンド fetch
        （スレッド内で `GitRepository::open(&path)` を新規に開く）
  - [x] スレッド完了時に `tx.send(...)` の後、必ず `ctx.request_repaint()` を呼ぶ

## フェーズ4: UI（src/ui/toolbar.rs）

- [x] 「⟳ リモート取得」ボタンを追加し、クリックで `state.needs_fetch = true` をセット
- [x] `state.is_fetching` 中はボタンを無効化し、`ui.spinner()` を表示
- [x] `show_branch_selector` の `ComboBox` に、ローカルブランチ一覧の後ろへ区切りを挟んで
      `remote_branches` を追記表示（クリックしてもチェックアウトは行わない）

## フェーズ5: 動作確認

- [x] `cargo run` で実リポジトリ（本プロジェクト自身の worktree、origin を持つ）を開き、
      以下を手動確認する（スクリーンショットで確認済み）
  - [x] 「リモート取得」ボタンが表示される
  - [x] クリック中も他の操作がフリーズせず継続できる（クリック直後もスクリーンショット撮影・
        マウス操作を継続でき、UIスレッドが固まっていないことを確認）
  - [x] fetch 中はボタンが無効化・スピナー表示される（グレーアウト + 回転スピナーを確認）
  - [x] fetch 成功後、ドロップダウンにローカルと区別できる形でリモートブランチが表示される
        （区切り線の下に `origin/feature/add-mit-license` 等が実際の GitHub origin から取得され表示された）
  - [x] ボタンを連打しても多重に fetch が走らない（`is_fetching` ガードをコードで担保、
        ボタンが無効化されている間はクリック自体が届かないため実質的に連打不可）
  - [x] リモートブランチをクリックしてもチェックアウトされない（`add_enabled(false, ...)` で
        disabled にしているため、egui の仕組み上クリックイベント自体が発生しない）

## フェーズ6: 品質チェックと修正

- [x] すべてのテストが通ることを確認
  - [x] `cargo test`（50件全て成功。ギュレル（implementation-validator）の指摘を受け、
        `apply_fetch_outcome` の抽出+単体テスト2件、`fetch_all_remotes` の異常系（到達不能リモート）
        テスト1件を追加）
- [x] リント・フォーマットに問題がないことを確認
  - [x] `cargo clippy --all-targets -- -D warnings`（警告ゼロ）
  - [x] `cargo fmt --check`（`src/git/remote.rs` に2箇所の崩れがあり `rustfmt src/git/remote.rs` で修正。
        修正後、今回変更した5ファイル（app.rs, toolbar.rs, repository.rs, mod.rs, remote.rs）は差分なし。
        `src/cli.rs` / `src/git/commit.rs` の差分は前回同様の既存ドリフトでスコープ外）
- [x] ビルドが成功することを確認
  - [x] `cargo build`

## フェーズ7: ドキュメント更新

- [x] `docs/architecture.md` を更新
  - [x] 「ネットワーク接続なし」制約を「ネットワーク接続は fetch など明示的なリモート操作時のみ」に修正
  - [x] 新章「並行処理モデル」を追加（UIスレッドは決してブロックしない原則、
        `std::thread` + `mpsc` パターン、git2 の `Repository::open` し直し規約、認証委譲方針）
  - [x] `docs/product-requirements.md` の「ブランチ操作」進捗欄も更新
- [x] README.md を更新（必要に応じて）（前回同様、README はタグラインであり機能網羅リストではないため更新不要と判断）
- [x] 実装後の振り返りを記録（別ファイル `retrospective.md` に記録 → モード3）

---

> **振り返りについて**: 実装後の振り返りはこのファイルではなく、同じディレクトリの
> `retrospective.md` に記録する（テンプレート: `.claude/skills/steering/templates/retrospective.md`）。
> 全タスクが `[x]` になったことを確認してから作成すること。
