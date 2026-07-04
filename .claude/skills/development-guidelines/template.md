# 開発ガイドライン (Development Guidelines)

## コーディング規約

### 命名規則

#### 変数・関数

**Rust**:
```rust
// ✅ 良い例
let user_profile_data = fetch_user_profile();
fn calculate_total_price(items: &[CartItem]) -> u32 { 0 }

// ❌ 悪い例
let data = fetch();
fn calc(arr: &[Box<dyn std::any::Any>]) -> u32 { 0 }
```

**原則**:
- 変数: snake_case、名詞または名詞句
- 関数: snake_case、動詞で始める
- 定数: UPPER_SNAKE_CASE
- Boolean: `is_`, `has_`, `should_`で始める

#### 構造体・トレイト

```rust
// 構造体: PascalCase、名詞
struct TaskManager { }
struct UserAuthenticationService { }

// トレイト: PascalCase
trait TaskRepository { }

// enum: PascalCase(バリアントもPascalCase)
enum TaskStatus {
    Todo,
    InProgress,
    Completed,
}
```

### コードフォーマット

**ツール**: `rustfmt`(`cargo fmt` で実行。`rustfmt.toml` は作成せずデフォルト設定を使用)

**インデント**: スペース4つ(rustfmtデフォルト)

**行の長さ**: 最大100文字(rustfmtデフォルト)

**コミット前に必ず実行**:
```bash
cargo fmt
cargo clippy -- -D warnings
```

### コメント規約

**関数・構造体のドキュメント**:
```rust
/// タスクの合計数を計算する
///
/// # Arguments
/// * `tasks` - 計算対象のタスク配列
/// * `filter` - フィルター条件(オプション)
///
/// # Errors
/// タスク配列が不正な場合は `ValidationError` を返す
fn count_tasks(tasks: &[Task], filter: Option<&TaskFilter>) -> Result<usize, ValidationError> {
    // 実装
    todo!()
}
```

**インラインコメント**:
```rust
// ✅ 良い例: なぜそうするかを説明
// キャッシュを無効化して、最新データを取得
cache.clear();

// ❌ 悪い例: 何をしているか(コードを見れば分かる)
// キャッシュをクリアする
cache.clear();
```

### エラーハンドリング

**原則**:
- 予期されるエラー: 適切なエラー型(enum)を定義
- 予期しないエラー: `Result` で上位に伝播
- `unwrap()`/`expect()` は本番コードに書かない(テストコードのみ許容)

**例**:
```rust
// エラー型定義
#[derive(Debug)]
struct ValidationError {
    field: String,
    message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "検証エラー [{}]: {}", self.field, self.message)
    }
}

// エラーハンドリング
match task_service.create(data) {
    Ok(task) => task,
    Err(ValidationError { field, message }) => {
        eprintln!("検証エラー [{field}]: {message}");
        // ユーザーにフィードバック
        return Err(ValidationError { field, message });
    }
}
```

## Git運用ルール

### ブランチ戦略

**ブランチ種別**:
- `main`: 本番環境にデプロイ可能な状態
- `develop`: 開発の最新状態
- `feature/[機能名]`: 新機能開発
- `fix/[修正内容]`: バグ修正
- `refactor/[対象]`: リファクタリング

**フロー**:
```
main
  └─ develop
      ├─ feature/task-management
      ├─ feature/user-auth
      └─ fix/task-validation
```

### コミットメッセージ規約

**フォーマット**:
```
<type>(<scope>): <subject>

<body>

<footer>
```

**Type**:
- `feat`: 新機能
- `fix`: バグ修正
- `docs`: ドキュメント
- `style`: コードフォーマット
- `refactor`: リファクタリング
- `test`: テスト追加・修正
- `chore`: ビルド、補助ツール等

**例**:
```
feat(task): タスクの優先度設定機能を追加

ユーザーがタスクに優先度(高/中/低)を設定できるようにしました。
- Taskモデルにpriorityフィールドを追加
- CLIに--priorityオプションを追加
- 優先度によるソート機能を実装

Closes #123
```

### プルリクエストプロセス

**作成前のチェック**:
- [ ] 全てのテストがパス
- [ ] Lintエラーがない
- [ ] 型チェックがパス
- [ ] 競合が解決されている

**PRテンプレート**:
```markdown
## 概要
[変更内容の簡潔な説明]

## 変更理由
[なぜこの変更が必要か]

## 変更内容
- [変更点1]
- [変更点2]

## テスト
- [ ] ユニットテスト追加
- [ ] 手動テスト実施

## スクリーンショット(該当する場合)
[画像]

## 関連Issue
Closes #[Issue番号]
```

**レビュープロセス**:
1. セルフレビュー
2. 自動テスト実行
3. レビュアーアサイン
4. レビューフィードバック対応
5. 承認後マージ

## テスト戦略

### テストの種類

#### ユニットテスト

**対象**: 個別の関数・クラス

**カバレッジ目標**: [80/90/100]%

**例**(各ソースファイル末尾の `#[cfg(test)] mod tests`、Rust の慣習):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_returns_task_when_data_is_valid() {
        let repository = InMemoryTaskRepository::new();
        let task = task_service::create(&repository, CreateTaskData {
            title: "テストタスク".to_string(),
            description: "説明".to_string(),
        }).unwrap();

        assert_eq!(task.title, "テストタスク");
    }

    #[test]
    fn create_returns_validation_error_when_title_is_empty() {
        let repository = InMemoryTaskRepository::new();
        let result = task_service::create(&repository, CreateTaskData {
            title: String::new(),
            description: String::new(),
        });

        assert!(matches!(result, Err(ValidationError { .. })));
    }
}
```

#### 統合テスト

**対象**: 複数コンポーネントの連携。`tests/` ディレクトリに配置

**例**:
```rust
// tests/task_crud.rs
#[test]
fn task_crud_flow_creates_reads_updates_and_deletes() {
    let repository = TaskRepository::open_temp();

    // 作成
    let created = repository.create(&CreateTaskData { title: "テスト".to_string() }).unwrap();

    // 取得
    let found = repository.find_by_id(&created.id).unwrap();
    assert_eq!(found.title, "テスト");

    // 更新
    repository.update(&created.id, &UpdateTaskData { title: "更新後".to_string() }).unwrap();
    let updated = repository.find_by_id(&created.id).unwrap();
    assert_eq!(updated.title, "更新後");

    // 削除
    repository.delete(&created.id).unwrap();
    assert!(repository.find_by_id(&created.id).is_none());
}
```

#### E2Eテスト

**対象**: ユーザーシナリオ全体(egui のUIは自動テストが困難なため、本プロジェクトでは手動確認を基本とする)

**例**(CLI部分など自動化可能な範囲):
```rust
// tests/cli_flow.rs
#[test]
fn user_can_open_repository_and_see_commit_history() {
    let repo_path = setup_test_repo();
    let output = run_cli(&["--repo", repo_path.to_str().unwrap(), "log"]);
    assert!(output.contains("Initial commit"));
}
```

### テスト命名規則

**パターン**: `<対象>_<条件>_<期待結果>`

**例**:
```rust
// ✅ 良い例
fn create_empty_title_returns_validation_error() { }
fn find_by_id_existing_id_returns_task() { }
fn delete_non_existent_id_returns_not_found_error() { }

// ❌ 悪い例
fn test1() { }
fn works() { }
fn should_work_correctly() { }
```

### テスト用実装(モック)の使用

**原則**:
- 外部依存(ファイルシステム等)はテスト用の一時ディレクトリ・一時リポジトリを使う(`tempfile` クレート)
- `src/git/` レイヤーは実際の `git2` を使うテスト用リポジトリで検証する(モックしない)
- ビジネスロジックは実装をそのまま使用

**例**:
```rust
// トレイトを実装したインメモリ版でモック代わりにする
struct InMemoryTaskRepository {
    tasks: RefCell<HashMap<String, Task>>,
}

impl TaskRepository for InMemoryTaskRepository {
    fn find_by_id(&self, id: &str) -> Option<Task> {
        self.tasks.borrow().get(id).cloned()
    }
}
```

## コードレビュー基準

### レビューポイント

**機能性**:
- [ ] 要件を満たしているか
- [ ] エッジケースが考慮されているか
- [ ] エラーハンドリングが適切か

**可読性**:
- [ ] 命名が明確か
- [ ] コメントが適切か
- [ ] 複雑なロジックが説明されているか

**保守性**:
- [ ] 重複コードがないか
- [ ] 責務が明確に分離されているか
- [ ] 変更の影響範囲が限定的か

**パフォーマンス**:
- [ ] 不要な計算がないか
- [ ] メモリリークの可能性がないか
- [ ] データベースクエリが最適化されているか

**セキュリティ**:
- [ ] 入力検証が適切か
- [ ] 機密情報がハードコードされていないか
- [ ] 権限チェックが実装されているか

### レビューコメントの書き方

**建設的なフィードバック**:
```markdown
## ✅ 良い例
この実装だと、タスク数が増えた時にパフォーマンスが劣化する可能性があります。
代わりに、インデックスを使った検索を検討してはどうでしょうか？

## ❌ 悪い例
この書き方は良くないです。
```

**優先度の明示**:
- `[必須]`: 修正必須
- `[推奨]`: 修正推奨
- `[提案]`: 検討してほしい
- `[質問]`: 理解のための質問

## 開発環境セットアップ

### 必要なツール

| ツール | バージョン | インストール方法 |
|--------|-----------|-----------------|
| [ツール1] | [バージョン] | [コマンド] |
| [ツール2] | [バージョン] | [コマンド] |

### セットアップ手順

```bash
# 1. リポジトリのクローン
git clone [URL]
cd [project-name]

# 2. 依存関係のインストール
[インストールコマンド]

# 3. 環境変数の設定
cp .env.example .env
# .envファイルを編集

# 4. 開発サーバーの起動
[起動コマンド]
```

### 推奨開発ツール(該当する場合)

- [ツール1]: [説明]
- [ツール2]: [説明]