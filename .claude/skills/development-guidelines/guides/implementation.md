# 実装ガイド (Implementation Guide)

## Rust 規約

### 型定義

**所有権を意識した型の使用**:
```rust
// ✅ 良い例: 借用で十分な場面では参照を受け取る
fn count_items(items: &[String]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for item in items {
        *counts.entry(item.clone()).or_insert(0) += 1;
    }
    counts
}

// ❌ 悪い例: 所有権が不要なのに値を消費してしまう
fn count_items(items: Vec<String>) -> HashMap<String, usize> {
    // 呼び出し側は items を使い回せなくなる
    let mut counts = HashMap::new();
    for item in items {
        *counts.entry(item).or_insert(0) += 1;
    }
    counts
}
```

**型注釈の原則**:
```rust
// ✅ 良い例: 公開APIの引数・戻り値には明示的な型を書く
pub fn calculate_total(prices: &[u32]) -> u32 {
    prices.iter().sum()
}

// ❌ 悪い例: 内部の一時変数にまで過剰な型注釈をつける
pub fn calculate_total(prices: &[u32]) -> u32 {
    let sum: u32 = prices.iter().sum::<u32>() as u32; // 型推論で十分な箇所
    sum
}
```

**構造体 vs トレイト**:
```rust
// 構造体: データの集約
struct Task {
    id: String,
    title: String,
    completed: bool,
}

// 構造体の拡張(コンポジション)
struct ExtendedTask {
    task: Task,
    priority: Priority,
}

// enum: 限定された選択肢(TypeScriptのユニオン型に相当)
enum TaskStatus {
    Todo,
    InProgress,
    Completed,
}

// トレイト: 共通の振る舞いを定義
trait Repository<T> {
    fn find_by_id(&self, id: &str) -> Option<T>;
}
```

### 命名規則

**変数・関数**:
```rust
// 変数: snake_case、名詞
let user_name = "John";
let task_list: Vec<Task> = Vec::new();
let is_completed = true;

// 関数: snake_case、動詞で始める
fn fetch_user_data() { }
fn validate_email(email: &str) -> bool { true }
fn calculate_total_price(items: &[Item]) -> u32 { 0 }

// Boolean: is_, has_, should_, can_ で始める
let is_valid = true;
let has_permission = false;
let should_retry = true;
let can_delete = false;
```

**構造体・トレイト・enum**:
```rust
// 構造体: PascalCase、名詞
struct TaskManager { }
struct UserAuthenticationService { }

// トレイト: PascalCase
trait TaskRepository { }
trait UserProfile { }

// enum: PascalCase(バリアントもPascalCase)
enum TaskStatus {
    Todo,
    InProgress,
    Completed,
}
```

**定数**:
```rust
// UPPER_SNAKE_CASE
const MAX_RETRY_COUNT: u32 = 3;
const API_BASE_URL: &str = "https://api.example.com";
const DEFAULT_TIMEOUT_MS: u64 = 5000;
```

**ファイル名・モジュール名**:
```rust
// モジュール・ファイル: snake_case
// task_service.rs
// user_repository.rs

// 関数・ユーティリティ: snake_case
// format_date.rs
// validate_email.rs

// モジュールの公開型はファイル名と対応させる
// task.rs 内に pub struct Task を定義

// 定数専用モジュール: snake_case
// api_endpoints.rs
// error_messages.rs
```

### 関数設計

**単一責務の原則**:
```rust
// ✅ 良い例: 単一の責務
fn calculate_total_price(items: &[CartItem]) -> u32 {
    items.iter().map(|item| item.price * item.quantity).sum()
}

fn format_price(amount: u32) -> String {
    format!("¥{}", amount)
}

// ❌ 悪い例: 複数の責務
fn calculate_and_format_price(items: &[CartItem]) -> String {
    let total: u32 = items.iter().map(|item| item.price * item.quantity).sum();
    format!("¥{}", total)
}
```

**関数の長さ**:
- 目標: 20行以内
- 推奨: 50行以内
- 100行以上: リファクタリングを検討

**パラメータの数**:
```rust
// ✅ 良い例: 構造体でまとめる
struct CreateTaskOptions {
    title: String,
    description: Option<String>,
    priority: Option<Priority>,
    due_date: Option<DateTime<Utc>>,
}

fn create_task(options: CreateTaskOptions) -> Task {
    // 実装
    todo!()
}

// ❌ 悪い例: パラメータが多すぎる
fn create_task(
    title: String,
    description: String,
    priority: String,
    due_date: DateTime<Utc>,
    tags: Vec<String>,
    assignee: String,
) -> Task {
    // 実装
    todo!()
}
```

### エラーハンドリング

**カスタムエラー型**:
```rust
// エラー型の定義(enumで種類を表現)
#[derive(Debug)]
enum TaskError {
    Validation { field: String, message: String },
    NotFound { resource: String, id: String },
    Database(std::io::Error),
}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskError::Validation { field, message } => {
                write!(f, "{field}: {message}")
            }
            TaskError::NotFound { resource, id } => {
                write!(f, "{resource} not found: {id}")
            }
            TaskError::Database(e) => write!(f, "database error: {e}"),
        }
    }
}

impl std::error::Error for TaskError {}
```

**エラーハンドリングパターン**:
```rust
// ✅ 良い例: Result と ? 演算子で適切に伝播
fn get_task(repository: &dyn Repository<Task>, id: &str) -> Result<Task, TaskError> {
    let task = repository
        .find_by_id(id)
        .ok_or_else(|| TaskError::NotFound {
            resource: "Task".to_string(),
            id: id.to_string(),
        })?;

    Ok(task)
}

// ❌ 悪い例: unwrap でクラッシュさせる
fn get_task(repository: &dyn Repository<Task>, id: &str) -> Task {
    repository.find_by_id(id).unwrap() // 見つからない場合パニックする
}
```

**エラーメッセージ**:
```rust
// ✅ 良い例: 具体的で解決策を示す
return Err(TaskError::Validation {
    field: "title".to_string(),
    message: "タイトルは1-200文字で入力してください。現在の文字数: 250".to_string(),
});

// ❌ 悪い例: 曖昧で役に立たない
return Err(TaskError::Validation {
    field: "title".to_string(),
    message: "invalid".to_string(),
});
```

### 並行処理

**スレッド/チャネルの使用**:
```rust
// ✅ 良い例: 結果をチャネルで受け取り、エラーを握りつぶさない
fn fetch_user_tasks(user_id: &str) -> Result<Vec<Task>, TaskError> {
    let user = user_repository::find_by_id(user_id)?;
    let tasks = task_repository::find_by_user_id(&user.id)?;
    Ok(tasks)
}

// ❌ 悪い例: エラーをログに残さず握りつぶす
fn fetch_user_tasks(user_id: &str) -> Vec<Task> {
    user_repository::find_by_id(user_id)
        .and_then(|u| task_repository::find_by_user_id(&u.id).ok())
        .unwrap_or_default()
}
```

**並列処理**:
```rust
// ✅ 良い例: rayon 等で並列実行(重い計算のみ。I/Oバウンドはスレッドプールを検討)
use rayon::prelude::*;

fn fetch_multiple_users(ids: &[String]) -> Vec<Result<User, TaskError>> {
    ids.par_iter().map(|id| user_repository::find_by_id(id)).collect()
}

// ❌ 悪い例: 並列化できる処理を逐次実行
fn fetch_multiple_users(ids: &[String]) -> Vec<Result<User, TaskError>> {
    let mut users = Vec::new();
    for id in ids {
        users.push(user_repository::find_by_id(id)); // 遅い
    }
    users
}
```

## コメント規約

### ドキュメントコメント

**`///` 形式(公開APIにのみ付与)**:
```rust
/// タスクを作成する
///
/// # Arguments
/// * `data` - 作成するタスクのデータ
///
/// # Errors
/// タイトルが空の場合は `TaskError::Validation` を返す
///
/// # Examples
/// ```
/// let task = create_task(CreateTaskData {
///     title: "新しいタスク".to_string(),
///     priority: Priority::High,
/// })?;
/// ```
pub fn create_task(data: CreateTaskData) -> Result<Task, TaskError> {
    // 実装
    todo!()
}
```

### インラインコメント

**良いコメント**:
```rust
// ✅ 理由を説明
// libgit2 は diff 生成時にファイル全体をメモリに載せるため、
// 1MB 超のファイルは OOM を避けるためにスキップする
if file_size > MAX_DIFF_SIZE_BYTES {
    return Err(TaskError::LargeFile(file_size));
}

// ✅ 複雑なロジックを説明
// Kadane のアルゴリズムで最大部分配列和を計算
// 時間計算量: O(n)
let mut max_so_far = arr[0];
let mut max_ending_here = arr[0];

// ✅ TODO・FIXMEを活用
// TODO: キャッシュ機能を実装 (Issue #123)
// FIXME: 大量データでパフォーマンス劣化 (Issue #456)
// HACK: 一時的な回避策、後でリファクタリング必要
```

**悪いコメント**:
```rust
// ❌ コードの内容を繰り返すだけ
// iを1増やす
i += 1;

// ❌ 古い情報
// このコードは2020年に追加された (不要な情報)

// ❌ コメントアウトされたコード
// let old_implementation = || { ... };  // 削除すべき
```

## セキュリティ

### 入力検証

```rust
// ✅ 良い例: 厳密な検証
fn validate_email(email: &str) -> Result<(), TaskError> {
    if email.is_empty() {
        return Err(TaskError::Validation {
            field: "email".to_string(),
            message: "メールアドレスは必須です".to_string(),
        });
    }

    if !email.contains('@') {
        return Err(TaskError::Validation {
            field: "email".to_string(),
            message: "メールアドレスの形式が不正です".to_string(),
        });
    }

    if email.len() > 254 {
        return Err(TaskError::Validation {
            field: "email".to_string(),
            message: "メールアドレスが長すぎます".to_string(),
        });
    }

    Ok(())
}

// ❌ 悪い例: 検証なし
fn validate_email(_email: &str) -> Result<(), TaskError> {
    Ok(()) // 検証なし
}
```

### 機密情報の管理

```rust
// ✅ 良い例: 環境変数から読み込み
let api_key = std::env::var("API_KEY")
    .map_err(|_| "API_KEY環境変数が設定されていません")?;

// ❌ 悪い例: ハードコード
let api_key = "sk-1234567890abcdef"; // 絶対にしない！
```

## パフォーマンス

### データ構造の選択

```rust
// ✅ 良い例: HashMap で O(1) アクセス
let user_map: HashMap<&str, &User> = users.iter().map(|u| (u.id.as_str(), u)).collect();
let user = user_map.get(user_id); // O(1)

// ❌ 悪い例: Vec の線形探索
let user = users.iter().find(|u| u.id == user_id); // O(n)
```

### ループの最適化

```rust
// ✅ 良い例: イテレータで不要な境界チェック・再計算を避ける
for item in items.iter() {
    process(item);
}

// ❌ 悪い例: インデックスアクセスで毎回境界チェックが走る
for i in 0..items.len() {
    process(&items[i]);
}
```

### メモ化

```rust
// 計算結果のキャッシュ
struct Cache {
    entries: HashMap<String, ExpensiveResult>,
}

impl Cache {
    fn get_or_compute(&mut self, input: &str) -> &ExpensiveResult {
        self.entries
            .entry(input.to_string())
            .or_insert_with(|| expensive_calculation(input))
    }
}
```

## テストコード

### テストの構造 (Given-When-Then)

配置は各ソースファイルの末尾(`#[cfg(test)] mod tests`、Rust の慣習)。

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_returns_task_with_valid_data() {
        // Given: 準備
        let repository = InMemoryTaskRepository::new();
        let task_data = CreateTaskData {
            title: "テストタスク".to_string(),
            description: "テスト用の説明".to_string(),
        };

        // When: 実行
        let result = create_task(&repository, task_data);

        // Then: 検証
        assert!(result.is_ok());
        let task = result.unwrap();
        assert_eq!(task.title, "テストタスク");
        assert_eq!(task.description, "テスト用の説明");
    }

    #[test]
    fn create_returns_validation_error_when_title_is_empty() {
        // Given: 準備
        let repository = InMemoryTaskRepository::new();
        let invalid_data = CreateTaskData {
            title: String::new(),
            description: String::new(),
        };

        // When/Then: 実行と検証
        let result = create_task(&repository, invalid_data);
        assert!(matches!(result, Err(TaskError::Validation { .. })));
    }
}
```

### テスト用リポジトリの作成

```rust
// ✅ 良い例: トレイトを実装したインメモリ実装でモック代わりにする
struct InMemoryTaskRepository {
    tasks: RefCell<HashMap<String, Task>>,
}

impl Repository<Task> for InMemoryTaskRepository {
    fn find_by_id(&self, id: &str) -> Option<Task> {
        self.tasks.borrow().get(id).cloned()
    }
}
```

## リファクタリング

### マジックナンバーの排除

```rust
// ✅ 良い例: 定数を定義
const MAX_RETRY_COUNT: u32 = 3;
const RETRY_DELAY_MS: u64 = 1000;

for i in 0..MAX_RETRY_COUNT {
    match fetch_data() {
        Ok(data) => return Ok(data),
        Err(e) if i < MAX_RETRY_COUNT - 1 => {
            std::thread::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS));
        }
        Err(e) => return Err(e),
    }
}

// ❌ 悪い例: マジックナンバー
for i in 0..3 {
    match fetch_data() {
        Ok(data) => return Ok(data),
        Err(_) if i < 2 => std::thread::sleep(std::time::Duration::from_millis(1000)),
        Err(e) => return Err(e),
    }
}
```

### 関数の抽出

```rust
// ✅ 良い例: 関数を抽出
fn process_order(order: &mut Order) -> Result<(), TaskError> {
    validate_order(order)?;
    calculate_total(order);
    apply_discounts(order);
    save_order(order)
}

fn validate_order(order: &Order) -> Result<(), TaskError> {
    if order.items.is_empty() {
        return Err(TaskError::Validation {
            field: "items".to_string(),
            message: "商品が選択されていません".to_string(),
        });
    }
    Ok(())
}

fn calculate_total(order: &mut Order) {
    order.total = order.items.iter().map(|i| i.price * i.quantity).sum();
}

// ❌ 悪い例: 長い関数
fn process_order(order: &mut Order) -> Result<(), TaskError> {
    if order.items.is_empty() {
        return Err(TaskError::Validation {
            field: "items".to_string(),
            message: "商品が選択されていません".to_string(),
        });
    }

    order.total = order.items.iter().map(|i| i.price * i.quantity).sum();

    if let Some(coupon) = &order.coupon {
        order.total -= (order.total as f64 * coupon.discount_rate) as u32;
    }

    save_order(order)
}
```

## チェックリスト

実装完了前に確認:

### コード品質
- [ ] 命名が明確で一貫している(Rust スタイルガイドに沿っているか)
- [ ] 関数が単一の責務を持っている
- [ ] マジックナンバーがない
- [ ] 型注釈が公開APIに適切に記載されている
- [ ] `unwrap()`/`expect()` が本番コードにない(テストコードのみ許容)
- [ ] エラーハンドリングが `Result<T, E>` で実装されている

### セキュリティ
- [ ] 入力検証が実装されている
- [ ] 機密情報がハードコードされていない
- [ ] ユーザー入力(パス等)が適切に扱われている

### パフォーマンス
- [ ] 適切なデータ構造を使用している
- [ ] 不要な計算・アロケーションを避けている
- [ ] ループが最適化されている

### テスト
- [ ] ユニットテストが書かれている(`#[cfg(test)] mod tests`)
- [ ] `cargo test` がパスする
- [ ] エッジケースがカバーされている

### ドキュメント
- [ ] 公開関数・構造体に `///` ドキュメントコメントがある
- [ ] 複雑なロジックに「なぜ」のコメントがある
- [ ] TODOやFIXMEが記載されている(該当する場合)

### ツール
- [ ] `cargo clippy -- -D warnings` がエラーなし
- [ ] `cargo fmt --check` がパスする
