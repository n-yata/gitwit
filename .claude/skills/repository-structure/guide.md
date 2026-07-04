# リポジトリ構造定義書作成ガイド

## 基本原則

### 1. 役割の明確化

各ディレクトリ(モジュール)は単一の明確な役割を持つべきです。

**悪い例**:
```
src/
├── stuff/           # 曖昧
├── misc/            # 雑多
└── utils/           # 汎用的すぎる
```

**良い例**:
```
src/
├── git/             # Gitロジック(git2ラッパー)
├── ui/              # egui ウィジェット
├── app.rs           # AppState(ViewModel)
└── cli.rs           # CLI引数解析
```

### 2. レイヤー分離の徹底

アーキテクチャのレイヤー構造をディレクトリ構造に反映させます:

```
src/
├── ui/              # UIレイヤー(eguiウィジェット)
│   └── commit_list.rs
├── app.rs           # ViewModel / AppStateレイヤー
└── git/             # Gitロジックレイヤー(git2クレート)
    └── commit.rs
```

### 3. 技術要素ベースの分割(基本)

関連する技術要素ごとにモジュールを分割します:

**基本構造**:
```
src/
├── ui/              # eguiウィジェット
├── app.rs           # AppState
├── git/             # git2ラッパー
└── error.rs         # エラー型定義
```

**レイヤー構造との対応**:
```
UIレイヤー          → ui/
ViewModelレイヤー   → app.rs
Gitロジックレイヤー → git/
```

## ディレクトリ構造の設計

### レイヤー構造の表現

```rust
// 悪い例: 平坦な構造
src/
├── commit_list_widget.rs
├── commit_service.rs
├── commit_repository.rs
├── diff_widget.rs
├── diff_service.rs
└── diff_repository.rs

// 良い例: レイヤーを明確に
src/
├── ui/
│   ├── commit_list.rs
│   └── diff_view.rs
├── app.rs           # commit_service, diff_service 相当のロジックを集約
└── git/
    ├── commit.rs
    └── diff.rs
```

### テストの配置

**推奨構造(Rustの慣習)**:
```
project/
├── src/
│   └── git/
│       └── commit.rs    # 末尾に #[cfg(test)] mod tests を配置(ユニットテスト)
└── tests/
    └── git_integration.rs   # 統合テスト(実際のgit2リポジトリを使用)
```

**理由**:
- ユニットテストは対象コードと同じファイルに置くのが Rust の慣習(コンパイラが `#[cfg(test)]` でテストビルド時のみ含める)
- 統合テストは `tests/` ディレクトリに分離し、クレートの公開APIのみを経由してテストする
- `cargo test` で両方まとめて実行できる

## 命名規則のベストプラクティス

### ディレクトリ・モジュール名の原則

**1. 複数形/単数形は役割に応じて自然な方を使う**
```
✅ git/ (Gitロジック一式)
✅ ui/ (UIウィジェット一式)

❌ stuffs/
❌ helper/
```

**2. snake_case を使う**
```
✅ commit_list.rs
✅ diff_view.rs

❌ CommitList.rs
❌ diffView.rs
```

理由: Rust の公式スタイルガイド(RFC 430)でモジュール名は snake_case と定められている

**3. 具体的な名前を使う**
```
✅ validators/       # 入力検証
✅ formatters/       # データ整形
✅ parsers/          # データ解析

❌ utils/            # 汎用的すぎる
❌ helpers/          # 曖昧
❌ common/           # 意味不明
```

### ファイル名の原則

**1. モジュールファイル: snake_case**
```rust
// Gitロジックのモジュール
commit.rs
diff.rs

// UIウィジェットのモジュール
commit_list.rs
diff_view.rs
```

**2. 関数を集めたユーティリティファイル: snake_case + 動詞由来の名前**
```rust
// ユーティリティ関数
format_date.rs
validate_path.rs
```

**3. 型定義: モジュール名と対応させる**
```rust
// commit.rs 内に pub struct CommitInfo を定義
// diff.rs 内に pub struct DiffLine, pub enum DiffLineKind を定義
```

**4. 定数専用モジュール: snake_case**
```rust
// 定数定義
constants.rs
error_messages.rs
```

## 依存関係の管理

### レイヤー間の依存ルール

```rust
// ✅ 良い例: 上位レイヤーから下位レイヤーへの依存
// app.rs
use crate::git::GitRepository;

struct AppState {
    repository: GitRepository,
}

// ❌ 悪い例: 下位レイヤーから上位レイヤーへの依存
// git/commit.rs
use crate::app::AppState; // 禁止！ Gitロジックがアプリ状態に依存してはならない
```

### 循環依存の回避

**問題のあるコード**:
```rust
// git/commit.rs
use crate::git::diff::DiffService;

pub struct CommitService {
    diff_service: DiffService,
}

// git/diff.rs
use crate::git::commit::CommitService; // 循環依存！

pub struct DiffService {
    commit_service: CommitService,
}
```

**解決策1: トレイトで抽象化して依存方向を反転させる**
```rust
// git/traits.rs
pub trait DiffLookup {
    fn find_diff(&self, commit_id: &str) -> Option<DiffLine>;
}

// git/commit.rs
use crate::git::traits::DiffLookup;

pub struct CommitService<D: DiffLookup> {
    diff_lookup: D,
}
```

**解決策2: 共通の機能を別モジュールに抽出**
```rust
// git/repository.rs に共通ロジックを集約
pub struct GitRepository {
    inner: git2::Repository,
}

impl GitRepository {
    pub fn load_commits(&self, limit: usize) -> Result<Vec<CommitInfo>, GitError> { todo!() }
    pub fn load_diff(&self, commit_id: &str) -> Result<Vec<DiffLine>, GitError> { todo!() }
}
```

## スケーリング戦略

### 推奨構造

**標準パターン**:
```
src/
├── main.rs
├── app.rs           # AppState
├── cli.rs           # CLI引数解析
├── error.rs         # GitError等のエラー型
├── git/
│   ├── mod.rs
│   ├── commit.rs
│   └── diff.rs
└── ui/
    ├── mod.rs
    ├── commit_list.rs
    └── diff_view.rs
```

**理由**:
- レイヤーごとに責務が明確
- 後からのリファクタリングが不要
- 一人開発でも構造が把握しやすい

### モジュール分離のタイミング

**分離を検討する兆候**:
1. ファイルの行数が300行を超える
2. 関連する機能がまとまっている
3. 独立してテスト可能
4. 他の機能への依存が少ない

**分離の手順**:
```rust
// Before: 全て git.rs に配置
git.rs (800行)

// After: 機能ごとにモジュール化
git/
├── mod.rs
├── commit.rs        # コミット履歴取得
├── diff.rs          # 差分計算
└── repository.rs    # リポジトリオープン・状態管理
```

## 特殊なケースの対応

### 共有コードの配置

**共通ユーティリティの配置**
```
src/
├── error.rs         # 全レイヤー共通のエラー型
├── ui/
├── app.rs
└── git/
```

**ルール**:
- 本当に複数のレイヤーで使われるもののみ `error.rs` 等に置く
- 単一レイヤーでしか使わないものは各モジュール内に留める

### 設定ファイルの管理(該当する場合)

```
src/
└── config.rs        # アプリ設定(ウィンドウサイズ等)の読み込み・保存
```

### スクリプトの管理(該当する場合)

```
scripts/
└── register-context-menu.ps1   # Explorer右クリックメニュー登録スクリプト
```

## ドキュメント配置

### ドキュメントの種類と配置先

**プロジェクトルート**:
- `README.md`: プロジェクト概要
- `CLAUDE.md`: プロジェクトメモリ・開発ルール
- `LICENSE`: ライセンス

**docs/ ディレクトリ**:
- `product-requirements.md`: PRD
- `functional-design.md`: 機能設計書
- `architecture.md`: アーキテクチャ設計書
- `repository-structure.md`: 本ドキュメント
- `development-guidelines.md`: 開発ガイドライン
- `glossary.md`: 用語集

**ソースコード内**:
- `///` ドキュメントコメント: 公開関数・構造体の説明(`pub` のみ)

## チェックリスト

- [ ] 各ディレクトリの役割が明確に定義されている
- [ ] レイヤー構造がディレクトリに反映されている
- [ ] 命名規則(snake_case等)が一貫している
- [ ] テストコードの配置方針(ユニット/統合)が決まっている
- [ ] 依存関係のルール(UI→AppState→Gitの一方向)が明確である
- [ ] 循環依存がない
- [ ] スケーリング戦略が考慮されている
- [ ] 共有コードの配置ルールが定義されている
- [ ] 設定ファイルの管理方法が決まっている
- [ ] ドキュメントの配置場所が明確である
