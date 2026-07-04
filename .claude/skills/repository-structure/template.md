# リポジトリ構造定義書 (Repository Structure Document)

## プロジェクト構造

```
project-root/
├── src/                   # ソースコード
│   ├── [layer1]/          # [説明]
│   ├── [layer2]/          # [説明]
│   └── [layer3]/          # [説明]
├── tests/                 # 統合テスト(ユニットテストは各ソースファイル内)
├── docs/                  # プロジェクトドキュメント
├── scripts/               # 補助スクリプト
└── Cargo.toml             # 依存関係・ビルド設定
```

## ディレクトリ詳細

### src/ (ソースコードディレクトリ)

#### [ディレクトリ1]

**役割**: [説明]

**配置ファイル**:
- [ファイルパターン1]: [説明]
- [ファイルパターン2]: [説明]

**命名規則**:
- [規則1]
- [規則2]

**依存関係**:
- 依存可能: [モジュール名]
- 依存禁止: [モジュール名]

**例**:
```
[モジュール名]/
├── mod.rs
├── [example_file1].rs
└── [example_file2].rs
```

#### [ディレクトリ2]

**役割**: [説明]

**配置ファイル**:
- [ファイルパターン1]: [説明]

**命名規則**:
- [規則1]

**依存関係**:
- 依存可能: [モジュール名]
- 依存禁止: [モジュール名]

### tests/ (統合テストディレクトリ)

**役割**: クレートの公開APIを経由した統合テストの配置

**構造**:
```
tests/
└── [feature]_integration.rs   # 機能単位でファイル分割
```

**命名規則**:
- パターン: `[対象機能]_integration.rs`
- 例: `git_integration.rs`

**ユニットテストについて**:
- ユニットテストは `tests/` に置かず、各ソースファイル末尾の `#[cfg(test)] mod tests` に配置する(Rustの慣習)

### docs/ (ドキュメントディレクトリ)

**配置ドキュメント**:
- `product-requirements.md`: プロダクト要求定義書
- `functional-design.md`: 機能設計書
- `architecture.md`: アーキテクチャ設計書
- `repository-structure.md`: リポジトリ構造定義書(本ドキュメント)
- `development-guidelines.md`: 開発ガイドライン
- `glossary.md`: 用語集

### scripts/ (スクリプトディレクトリ - 該当する場合)

**配置ファイル**:
- ビルド補助スクリプト
- 開発補助スクリプト(例: `.ps1`, `.sh`)

## ファイル配置規則

### ソースファイル

| ファイル種別 | 配置先 | 命名規則 | 例 |
|------------|--------|---------|-----|
| [種別1] | [ディレクトリ] | [規則] | [例] |
| [種別2] | [ディレクトリ] | [規則] | [例] |

### テストファイル

| テスト種別 | 配置先 | 命名規則 | 例 |
|-----------|--------|---------|-----|
| ユニットテスト | 各ソースファイル内 `#[cfg(test)] mod tests` | 関数名で表現 | `format_relative_time_minutes` |
| 統合テスト | tests/ | `[機能]_integration.rs` | `git_integration.rs` |

### 設定ファイル

| ファイル種別 | 配置先 | 命名規則 |
|------------|--------|---------|
| 依存関係・ビルド設定 | プロジェクトルート | `Cargo.toml` |
| フォーマット設定(該当する場合) | プロジェクトルート | `rustfmt.toml` |
| Lint設定(該当する場合) | プロジェクトルート | `clippy.toml` |

## 命名規則

### ディレクトリ・モジュール名

- **レイヤーモジュール**: snake_case
  - 例: `git/`, `ui/`
- **機能モジュール**: snake_case
  - 例: `commit_list/`, `diff_view/`

### ファイル名

- **モジュールファイル**: snake_case
  - 例: `commit.rs`, `diff.rs`
- **関数を集めたファイル**: snake_case
  - 例: `format_date.rs`, `validate_path.rs`
- **定数専用ファイル**: snake_case
  - 例: `constants.rs`, `error_messages.rs`

### テストファイル名

- 統合テスト: `[機能]_integration.rs`
- ユニットテスト関数名: `<対象関数>_<条件>_<期待結果>`

## 依存関係のルール

### レイヤー間の依存

```
UIレイヤー
    ↓ (OK)
ViewModel / AppStateレイヤー
    ↓ (OK)
Gitロジックレイヤー
```

**禁止される依存**:
- Gitロジックレイヤー → AppStateレイヤー (❌)
- Gitロジックレイヤー → UIレイヤー (❌)
- AppStateレイヤー → UIレイヤー (❌)

### モジュール間の依存

**循環依存の禁止**:
```rust
// ❌ 悪い例: 循環依存
// file_a.rs
use crate::file_b::func_b;

// file_b.rs
use crate::file_a::func_a;  // 循環依存
```

**解決策**:
```rust
// ✅ 良い例: 共通モジュールの抽出
// shared.rs
pub struct SharedType { /* ... */ }

// file_a.rs
use crate::shared::SharedType;

// file_b.rs
use crate::shared::SharedType;
```

## スケーリング戦略

### 機能の追加

新しい機能を追加する際の配置方針:

1. **小規模機能**: 既存モジュールに配置
2. **中規模機能**: レイヤー内にサブモジュールを作成
3. **大規模機能**: 独立したモジュールとして分離

**例**:
```
src/
├── git/
│   ├── mod.rs
│   ├── commit.rs           # 既存機能
│   └── commit/             # 中規模機能の分離
│       ├── mod.rs
│       ├── history.rs
│       └── filter.rs
```

### ファイルサイズの管理

**ファイル分割の目安**:
- 1ファイル: 300行以下を推奨
- 300-500行: リファクタリングを検討
- 500行以上: 分割を強く推奨

**分割方法**:
```rust
// 悪い例: 1ファイルに全機能
// git.rs (800行)

// 良い例: 責務ごとに分割
// git/commit.rs (200行) - コミット履歴取得
// git/diff.rs (150行) - 差分計算
// git/repository.rs (100行) - リポジトリオープン・状態管理
```

## 特殊ディレクトリ

### .steering/ (ステアリングファイル)

**役割**: 特定の開発作業における「今回何をするか」を定義

**構造**:
```
.steering/
└── [YYYYMMDD]-[task-name]/
    ├── requirements.md      # 今回の作業の要求内容
    ├── design.md            # 変更内容の設計
    └── tasklist.md          # タスクリスト
```

**命名規則**: `20260702-add-commit-history` 形式

### .claude/ (Claude Code設定)

**役割**: Claude Code設定とカスタマイズ

**構造**:
```
.claude/
├── commands/                # スラッシュコマンド
├── skills/                  # タスクモード別スキル
└── agents/                  # サブエージェント定義
```

## 除外設定

### .gitignore

プロジェクトで除外すべきファイル:
- `target/` (Cargoのビルド出力)
- `.env`
- `*.log`
- `.DS_Store`

### rustfmt/clippy 対象外の設定(該当する場合)

`Cargo.toml` の `[workspace]`/`exclude` や、ファイル冒頭の `#[rustfmt::skip]` 等で個別に除外する。
