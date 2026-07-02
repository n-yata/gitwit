# 開発ガイドライン (Development Guidelines)

## コーディング規約

### 命名規則

Rust の公式スタイルガイド（RFC 430）に従う。

| 対象 | 規則 | 例 |
|------|------|-----|
| 構造体・列挙型・トレイト | `PascalCase` | `CommitInfo`, `DiffLineKind` |
| 関数・メソッド・変数 | `snake_case` | `load_commits`, `short_id` |
| 定数 | `UPPER_SNAKE_CASE` | `MAX_DIFF_SIZE_BYTES` |
| モジュール | `snake_case` | `git`, `diff` |
| 型パラメータ | 単一大文字または短い `PascalCase` | `T`, `E` |

**原則**:
- 変数・関数名は何をするか/何であるかが分かる名前にする
- Boolean 変数は `is_`, `has_`, `should_` で始める（例: `is_binary`, `is_loading`）
- 省略はしない（`msg` → `message`, `err` → `error` を推奨。標準慣習の `e`・`i`・`n` は許容）

### コードフォーマット

**ツール**: `rustfmt`（`cargo fmt` で実行）

**設定**: `rustfmt.toml` は作成せず、デフォルト設定を使用。

**インデント**: スペース 4 つ（rustfmt デフォルト）

**行の長さ**: 100 文字（rustfmt デフォルトの 100）

**コミット前に必ず実行**:
```bash
cargo fmt
cargo clippy -- -D warnings
```

### コメント規約

コメントは「なぜ（WHY）」を書く。「何をしているか（WHAT）」はコードを読めば分かる。

```rust
// ✅ 良い例: なぜそうするかを説明
// libgit2 は diff 生成時にファイル全体をメモリに載せるため、
// 1MB 超のファイルは OOM を避けるためにスキップする
if file_size > MAX_DIFF_SIZE_BYTES {
    return Err(GitError::LargeFile(file_size));
}

// ❌ 悪い例: コードを読めば分かることを書いている
// ファイルサイズが上限を超えているかチェックする
if file_size > MAX_DIFF_SIZE_BYTES {
    return Err(GitError::LargeFile(file_size));
}
```

**ドキュメントコメント（`///`）**: 公開 API（`pub` な関数・構造体）にのみ付与する。

### エラーハンドリング

**原則**:
- `unwrap()` / `expect()` は本番コードに書かない。テストコードのみ許容
- エラーは `Result<T, E>` で伝播する
- `?` 演算子を積極的に使う
- UI に見せるエラーメッセージは `GitError` → `String` に変換してから `AppState.error_message` にセット

```rust
// ✅ 良い例
fn load_commits(&self, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
    let mut revwalk = self.inner.revwalk().map_err(GitError::Git2)?;
    revwalk.push_head().map_err(GitError::Git2)?;
    // ...
}

// ❌ 悪い例
fn load_commits(&self, limit: usize) -> Vec<CommitInfo> {
    let mut revwalk = self.inner.revwalk().unwrap(); // クラッシュする可能性
    // ...
}
```

**カスタムエラー型**:
```rust
#[derive(Debug)]
enum GitError {
    Git2(git2::Error),          // libgit2 由来のエラー
    NotARepository(PathBuf),    // Git リポジトリでないパス
    LargeFile(u64),             // サイズ超過ファイル
    BinaryFile,                 // バイナリファイル
}

impl fmt::Display for GitError {
    // ユーザー向けメッセージに変換
}
```

## Git 運用ルール

CLAUDE.md の「Git 運用ルール」が正本。ここでは開発者向けの補足を記載する。

### ブランチ戦略

- **`main`**: 常に動作する状態を保つ。直接コミット禁止
- **`feature/<説明>`**: 全ての実装はこのブランチで行う

### コミットメッセージ規約

**フォーマット**: Conventional Commits に準拠
```
<type>(<scope>): <subject>

<body（任意）>
```

**type**:
| type | 用途 |
|------|------|
| `feat` | 新機能 |
| `fix` | バグ修正 |
| `docs` | ドキュメントのみの変更 |
| `refactor` | 動作変更なしのコード改善 |
| `test` | テスト追加・修正 |
| `chore` | ビルド設定・依存更新等 |

**scope（任意）**: 変更範囲を示す（`git`, `ui`, `config` 等）

**例**:
```
feat(git): コミット履歴の一覧取得を実装

git2::Revwalk を使って HEAD から最大 1000 件のコミットを
CommitInfo に変換して返す。
```

### PR チェックリスト

PR 作成前に以下を全て確認する:

- [ ] `cargo fmt` を実行した
- [ ] `cargo clippy -- -D warnings` がエラーなし
- [ ] `cargo test` が全テストパス
- [ ] セキュリティレビュー（クルトワ）完了
- [ ] `retrospective.md` を作成した（学びがある場合）

## テスト戦略

### テストの種類と方針

#### ユニットテスト（`#[cfg(test)] mod tests`）

**配置**: 各ソースファイルの末尾（Rust の慣習）

**対象**: `src/git/` 内の純粋な変換関数を優先してテストする

**カバレッジ目標**: `src/git/` レイヤーは 80% 以上

```rust
// src/git/commit.rs の末尾
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_relative_time_minutes() {
        let now = chrono::Utc::now().timestamp();
        let twenty_min_ago = now - 20 * 60;
        assert_eq!(format_relative_time(twenty_min_ago), "20分前");
    }

    #[test]
    fn format_relative_time_days() {
        let now = chrono::Utc::now().timestamp();
        let three_days_ago = now - 3 * 24 * 60 * 60;
        assert_eq!(format_relative_time(three_days_ago), "3日前");
    }
}
```

**テスト命名規則**: `<対象関数>_<条件>_<期待結果>` または日本語で「〜の場合〜を返す」

#### 統合テスト（`tests/`）

**配置**: `tests/git_integration.rs`

**方針**: 一時ディレクトリに `git2` で実際のコミットを作成して検証する

```rust
// tests/git_integration.rs
use std::path::Path;
use tempfile::TempDir;

fn setup_test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    // テスト用コミットを追加...
    dir
}

#[test]
fn load_commits_returns_commits_in_reverse_order() {
    let dir = setup_test_repo();
    let repo = GitRepository::open(dir.path()).unwrap();
    let commits = repo.load_commits(100).unwrap();
    assert!(!commits.is_empty());
    // 新しいコミットが先頭にあることを確認
}
```

### モック方針

- **`src/git/` レイヤー**: 実際の `git2` を使うテスト用リポジトリで検証（モックしない）
- **UI レイヤー**: egui のテストは手動確認とする（自動テストは困難なため）

## コードレビュー基準

### レビューポイント

**機能性**:
- [ ] PRD・機能設計書の受け入れ条件を満たしているか
- [ ] エラーケースが処理されているか（`unwrap` がないか）
- [ ] パフォーマンス要件（200ms・60fps）を損なう実装がないか

**可読性**:
- [ ] 命名が Rust スタイルガイドに沿っているか
- [ ] 複雑なロジックに「なぜ」のコメントがあるか

**保守性**:
- [ ] レイヤーの依存方向を守っているか（UI → AppState → Git の方向のみ）
- [ ] 1ファイル 300 行以下か

**セキュリティ**:
- [ ] `unwrap()` が本番コードにないか
- [ ] ユーザー入力（パス）が適切に扱われているか

## 開発環境セットアップ

### 必要なツール

| ツール | バージョン | インストール方法 |
|--------|-----------|-----------------|
| Rust（rustup） | 最新安定版 | `winget install Rustlang.Rustup` |
| Git | 任意の最新版 | `winget install Git.Git` |
| Visual Studio C++ Build Tools | 2022 | Rust for Windows に必要 |

### セットアップ手順

```powershell
# 1. リポジトリのクローン
git clone <URL>
cd git-client

# 2. Rust が入っているか確認
rustc --version
cargo --version

# 3. ビルド（初回は時間がかかる。libgit2 のコンパイルを含む）
cargo build

# 4. 実行
cargo run

# 5. テスト
cargo test

# 6. Lint・フォーマット確認
cargo fmt --check
cargo clippy -- -D warnings
```

### 推奨 VSCode 拡張

- **rust-analyzer**: Rust の IntelliSense・補完（必須）
- **Even Better TOML**: `Cargo.toml` のシンタックスハイライト
- **Error Lens**: インラインエラー表示
