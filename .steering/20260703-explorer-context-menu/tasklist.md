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

## フェーズ1: Git ロジック拡張

- [x] `src/git/commit.rs`: `build_commit_info` 共通ヘルパーを切り出す
  - [x] 既存 `load_commits` をリファクタリングして共通ヘルパー経由にする
- [x] `src/git/commit.rs`: `load_commits_for_path(repo, limit, path)` を実装
  - [x] revwalkしながら親コミットとの差分にpathspecを設定し、delta有無で採否判定
  - [x] 初回コミット(親なし)は空ツリーとの差分で判定
- [x] `src/git/repository.rs`: `load_commits_for_path` を `GitRepository` に追加(委譲)
- [x] ~~`src/git/repository.rs`: `GitRepository::from_git2` を追加~~（実装方針変更により不要: cli.rsはgit2::Repository::discoverでworkdirを求めるだけに留め、実際のGitRepository構築は既存のopen()に一本化したためAPI追加が不要になった）

## フェーズ2: CLI引数解決

- [x] `src/cli.rs` を新規作成
  - [x] `CliTarget { repo_root: PathBuf, file_filter: Option<String> }` 定義
  - [x] `resolve_target(raw_path: &Path) -> Result<CliTarget, GitError>` 実装
    - [x] `canonicalize` でファイル/フォルダ判定、存在しない場合はエラー
    - [x] `git2::Repository::discover` でリポジトリルート検出
    - [x] ファイル指定時、workdir からの相対パスを `/` 区切りで計算
- [x] `main.rs` を非公開モジュールとして `cli` を登録
- [x] ユニットテスト: ファイル指定時に相対パスが正しく計算される
- [x] ユニットテスト: フォルダ指定時に `file_filter` が `None` になる
- [x] ユニットテスト: リポジトリ外パスでエラーになる

## フェーズ3: main.rs / App 統合

- [x] `src/main.rs`: `std::env::args().nth(1)` を取得し `App::new` に渡す(パス自体を渡し、解決はAppState::new内で実施)
- [x] `src/app.rs`: `AppState` に `file_filter: Option<String>` を追加
- [x] `src/app.rs`: `AppState::new` の引数に CLI解決結果を追加し、`Some` の場合は `repo_path`/`path_input`/`file_filter`/`needs_load` を上書き
- [x] `src/app.rs`: CLI解決が `Err` の場合は `error_message` にセットしつつ、通常の config フォールバックを維持
- [x] `src/app.rs`: `App::new` のシグネチャ変更を `main.rs` の呼び出し側に反映
- [x] `src/app.rs`: `load_repo()` を `file_filter` の有無で `load_commits` / `load_commits_for_path` に分岐

## フェーズ4: UI(フィルタ表示・解除)

- [x] `src/ui/toolbar.rs`: `state.file_filter` が `Some` のときフィルタ表示ラベルを追加
- [x] `src/ui/toolbar.rs`: フィルタ解除ボタンを追加(押下で `file_filter = None; needs_load = true;`)

## フェーズ5: Explorer 右クリックメニュー登録スクリプト

- [x] `scripts/register-context-menu.ps1` を新規作成
  - [x] スクリプト位置からexeパスを解決(`..\target\release\gitwit.exe`)、存在しなければエラー終了
  - [x] `HKCU:\Software\Classes\*\shell\Gitwit` を登録(MUIVerb, Icon, command)
  - [x] `HKCU:\Software\Classes\Directory\shell\Gitwit` を登録
  - [x] `HKCU:\Software\Classes\Directory\Background\shell\Gitwit` を登録(commandは `%V`)
- [x] `scripts/unregister-context-menu.ps1` を新規作成
  - [x] 上記3キーを `Remove-Item -Recurse` (存在チェック付き)で削除

## フェーズ6: 品質チェックと修正

- [x] `cargo test` が全て通ることを確認（8 passed）
- [x] `cargo check` でエラーがないことを確認
- [x] `cargo clippy` で警告がないことを確認(既存の許容済みlintは踏襲)
- [x] `cargo build --release` が成功することを確認
- [x] セキュリティレビュー(security-engineer)を実施し、Critical/Highがあれば修正（Critical/Highなし。Low2件は実害なし・性能留意点として記録のみ）

## フェーズ7: ドキュメント更新

- [x] `docs/repository-structure.md` に `src/cli.rs`, `scripts/` を反映(必要に応じて)
- [ ] 実装後の振り返りを記録（別ファイル `retrospective.md` に記録 → モード3）

---

> **振り返りについて**: 実装後の振り返りはこのファイルではなく、同じディレクトリの
> `retrospective.md` に記録する（テンプレート: `.claude/skills/steering/templates/retrospective.md`）。
> 全タスクが `[x]` になったことを確認してから作成すること。
