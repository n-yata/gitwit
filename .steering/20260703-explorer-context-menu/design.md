# 設計書

## アーキテクチャ概要

既存のレイヤー構成(UI → App State → Git ロジック)を維持しつつ、起動時に1回だけ動く「CLI引数解決」ステップを新設する。

```
main.rs
  ├─ 起動時: std::env::args() から対象パス文字列を取得
  ├─ cli.rs: resolve_target(path) でリポジトリルート + (任意)ファイルフィルタに解決
  └─ App::new(cc, resolved_target) へ渡す

App::new
  └─ AppState::new(resolved_target)
       ├─ resolved_target が Some なら repo_path / path_input / file_filter を設定し needs_load = true
       └─ None なら従来通り config の last_repo_path を使用

App::load_repo()
  └─ state.file_filter が Some なら GitRepository::load_commits_for_path()
     None なら従来の GitRepository::load_commits()

git/commit.rs
  └─ load_commits_for_path(repo, limit, path) を追加
     (revwalk しながら各コミットの diff にファイルが含まれるかを判定)

scripts/register-context-menu.ps1 / unregister-context-menu.ps1
  └─ HKCU\Software\Classes 配下に静的コンテキストメニューを追加/削除する独立スクリプト
     (アプリ本体とは疎結合。exe パスは実行時にスクリプトから解決)
```

## コンポーネント設計

### 1. `src/cli.rs` (新規)

**責務**:
- CLIから渡された生パス文字列を、リポジトリルートと任意のファイルフィルタパスに解決する
- ファイル/フォルダの存在確認、リポジトリ検出の失敗を `Result` で返す

**実装の要点**:
- `git2::Repository::discover(path)` を使い、指定パス(またはその親)から上方向に `.git` を探索してリポジトリを取得する。単純な `Repository::open` は完全一致パスしか受け付けないため使わない。
- 入力パスがファイルかどうかは `std::fs::metadata` で判定する。
- ファイル指定の場合、リポジトリの `workdir()` からの相対パスを計算し、区切り文字を `/` に統一する(git の内部パス表現に合わせる)。
- 戻り値: `CliTarget { repo_root: PathBuf, file_filter: Option<String> }`
- パスが存在しない/リポジトリが見つからない場合は `GitError` を返し、呼び出し側(App)がトーストと同じ `error_message` 表示に流用する。

**シグネチャ**:
```rust
pub struct CliTarget {
    pub repo_root: PathBuf,
    pub file_filter: Option<String>,
}

pub fn resolve_target(raw_path: &Path) -> Result<CliTarget, GitError>
```

### 2. `src/git/commit.rs` の拡張

**責務**:
- 指定ファイルパスが変更されたコミットのみを対象に、コミット一覧を構築する

**実装の要点**:
- 既存 `load_commits` と同様に `revwalk` で HEAD から辿るが、各コミットについて親との差分 (`diff_tree_to_tree` に `pathspec` を設定) を取り、delta が1件以上あれば採用する。
- 初回コミット(親なし)は空ツリーとの差分で判定する(`diff.rs` の `load_diff_files` と同じパターン)。
- `limit` は「マッチしたコミット数の上限」として扱う(スキャンする総コミット数の上限ではない)。巨大リポジトリでの探索コストはスコープ外(MVPでは許容)。
- 関数シグネチャ: `pub(super) fn load_commits_for_path(repo: &Repository, limit: usize, path: &str) -> Result<Vec<CommitInfo>, GitError>`
- 戻り値の `CommitInfo` 構築処理(短縮ID・author・時刻・refs収集)は `load_commits` と重複するため、共通ヘルパー `build_commit_info(repo, oid, refs_map) -> Result<CommitInfo, GitError>` に切り出して両関数から呼ぶ。

### 3. `src/git/repository.rs` の拡張

**責務**:
- `GitRepository` に `load_commits_for_path` を追加し、`commit.rs` の新関数へ委譲する

**実装の要点**:
- `cli.rs` は `git2::Repository::discover` で生の `git2::Repository` を取得して `workdir()` から相対パスを計算するだけに留め、実際にアプリが使う `GitRepository` は既存の `GitRepository::open(&repo_root)` を通常どおり呼んで構築する(discover で得た `Repository` をそのまま使い回す専用APIは追加せず、既存の `open` 経路に一本化してAPI面を増やさない)。

### 4. `src/app.rs` の拡張

**責務**:
- CLI解決結果を `AppState` の初期値に反映する
- ファイルフィルタの有無で読み込み関数を切り替える
- フィルタ表示・解除UIの状態を保持する

**実装の要点**:
- `AppState` に `pub file_filter: Option<String>` を追加。
- `AppState::new()` の引数に `cli_target: Option<CliTarget>` を追加。`Some` の場合、`repo_path`/`path_input`/`file_filter`/`needs_load` を上書きする。CLI解決が `Err` だった場合は `error_message` に格納し、通常の起動(config の last_repo_path)にフォールバックする。
- `App::load_repo()` 内で `self.state.file_filter` の有無により `repo.load_commits(..)` / `repo.load_commits_for_path(.., path)` を呼び分ける。
- フィルタ解除操作: `AppState` に `needs_clear_filter: bool` のようなフラグは不要で、UIから直接 `file_filter = None; needs_load = true;` をセットする単純な処理で足りる(他の `needs_*` フラグと同じパターン)。

### 5. `src/ui/toolbar.rs` の拡張

**責務**:
- フィルタ適用中であることの表示と解除ボタンの提供

**実装の要点**:
- 既存のツールバー(リポジトリパス入力欄がある想定)に、`state.file_filter` が `Some` のときだけ「履歴フィルタ: {path} ✕」のラベル+ボタンを追加表示する。
- ボタン押下で `state.file_filter = None; state.needs_load = true;` をセットする。

### 6. `src/main.rs` の拡張

**実装の要点**:
- `std::env::args().nth(1)` で最初の引数(パス文字列)を取得。
- 存在すれば `cli::resolve_target(Path::new(&arg))` を呼び、結果を `App::new` に渡す。
- `App::new` のシグネチャに `cli_target: Option<Result<CliTarget, GitError>>` を渡す(解決失敗もエラー表示に使うため `Result` のまま渡す)。

### 7. `scripts/register-context-menu.ps1` / `scripts/unregister-context-menu.ps1` (新規)

**責務**:
- Explorer への静的コンテキストメニュー登録・解除

**実装の要点**:
- 登録スクリプトはスクリプト自身の場所から `..\target\release\gitwit.exe` を解決し、存在しなければエラー終了する(ビルド済みバイナリが前提)。
- レジストリキー:
  - `HKCU:\Software\Classes\*\shell\Gitwit`
  - `HKCU:\Software\Classes\Directory\shell\Gitwit`
  - `HKCU:\Software\Classes\Directory\Background\shell\Gitwit`
  - 各 `\command` サブキーの既定値に `"<exeパス>" "%1"` (`Directory\Background` のみ `"%V"`)を設定する。
- `MUIVerb` に表示名「Gitwitで履歴を表示」、`Icon` に exe パスを設定してメニューに反映する。
- 解除スクリプトは上記3キーを `Remove-Item -Recurse` する(存在しない場合はスキップ)。
- **HKCUのみを操作し、HKLMや管理者権限は一切必要としない**(セキュリティ・実行容易性の両面で重要な制約)。
- スクリプトはユーザーが明示的に実行するものであり、アプリやビルドプロセスから自動実行しない。

## データフロー

### Explorerからファイルを右クリックして履歴表示

```
1. ユーザーが Explorer で対象ファイルを右クリックし「Gitwitで履歴を表示」をクリック
2. Windows が登録済みコマンド "<exe>" "<選択ファイルの絶対パス>" を起動
3. main.rs が argv[1] を取得し cli::resolve_target() を呼ぶ
4. resolve_target が git2::Repository::discover でリポジトリルートを検出し、
   ファイルパスをリポジトリルートからの相対パスに変換して CliTarget を返す
5. App::new が CliTarget を AppState に反映(repo_path, file_filter, needs_load=true)
6. update() の冒頭で needs_load が処理され、load_repo() が file_filter ありとして
   load_commits_for_path を呼ぶ
7. コミット一覧にフィルタ済み結果が表示され、ツールバーに「履歴フィルタ: <相対パス>」が出る
```

## エラーハンドリング戦略

新規エラーケースは既存 `GitError` で表現可能なため、新しいエラー型は追加しない。

- パスが存在しない: `std::fs::metadata` 失敗時、`GitError::NotARepository(path文字列 + 理由)` として返す。
- リポジトリが見つからない: `git2::Repository::discover` の `Err` をそのまま `GitError::from` (既存の `From<git2::Error>` 実装)に委譲する。
- いずれの場合も `App` 側で `state.error_message` に格納し、既存のエラーウィンドウ表示パターンをそのまま利用する。

## テスト戦略

### ユニットテスト
- `cli::resolve_target`: ファイルパス指定時に相対パスが正しく計算されること(テスト用の一時Gitリポジトリを `tempfile` 等を使わず、`git2::Repository::init` で都度作成してテストする)
- `cli::resolve_target`: フォルダパス指定時に `file_filter` が `None` になること
- `cli::resolve_target`: リポジトリ外のパスを渡した場合にエラーになること
- `commit::load_commits_for_path`: 指定ファイルを変更したコミットのみが返ること(変更してないコミットが除外されること)

### 統合テスト
- なし(GUI操作を伴う統合テストは本プロジェクトのMVPスコープ外。手動確認で代替)

## 依存ライブラリ

新規追加ライブラリなし。既存の `git2` の機能のみで実現する。

## ディレクトリ構造

```
src/
  cli.rs                 # 新規: CLI引数解決
  main.rs                # 変更: 引数取得とApp::newへの受け渡し
  app.rs                 # 変更: AppState拡張、load_repo分岐
  git/
    mod.rs               # 変更: cli.rs から使う型のexport見直し(必要なら)
    commit.rs            # 変更: load_commits_for_path, build_commit_info共通化
    repository.rs         # 変更: load_commits_for_path委譲、from_git2追加
  ui/
    toolbar.rs            # 変更: フィルタ表示・解除UI
scripts/
  register-context-menu.ps1    # 新規
  unregister-context-menu.ps1  # 新規
```

## 実装の順序

1. `git/commit.rs`: `build_commit_info` 共通化 + `load_commits_for_path` 追加
2. `git/repository.rs`: `load_commits_for_path` 委譲 + `from_git2` 追加
3. `cli.rs`: `resolve_target` 実装 + ユニットテスト
4. `main.rs`: CLI引数取得と `App::new` への受け渡し
5. `app.rs`: `AppState` 拡張、`load_repo` 分岐、CLI解決結果の反映
6. `ui/toolbar.rs`: フィルタ表示・解除UI
7. `scripts/register-context-menu.ps1` / `unregister-context-menu.ps1` 作成
8. 品質チェック(`cargo test`, `cargo check`, `cargo clippy`)
9. 手動確認(CLI引数起動、レジストリ登録・右クリック確認)

## セキュリティ考慮事項

- レジストリ登録は **HKCU(現在のユーザー)のみ**を操作し、管理者権限や HKLM への書き込みは行わない。
- コンテキストメニューのコマンドは `"<exeの絶対パス>" "%1"` という固定文字列をレジストリに書き込むのみで、シェル文字列展開(`cmd.exe /c` 等)を経由しないため、ファイル名にシェルメタ文字が含まれてもコマンドインジェクションは発生しない。
- CLIから受け取るパスは `Path` として扱い、シェルコマンドとして再解釈しない(直接ファイルシステムAPIに渡すのみ)。
- exeパスやリポジトリパスのハードコーディングはしない(スクリプトは相対解決、アプリはCLI引数/設定ファイルから取得)。

## パフォーマンス考慮事項

- `load_commits_for_path` は最悪ケースで全コミット履歴をスキャンする。MVPでは許容するが、将来的に大規模リポジトリで問題になれば `git2::Diff` のペイロード削減(`DiffOptions::pathspec` 済みなので実際の差分計算自体は軽い)や、非同期化・進捗表示を検討する。

## 将来の拡張性

- `--follow` 相当のリネーム追跡対応
- 複数ファイル選択時の複合フィルタ
- HKLM登録によるマシン全体への配布(インストーラー化)
