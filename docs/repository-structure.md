# リポジトリ構造定義書 (Repository Structure Document)

## プロジェクト構造

```
git-client/
├── src/                        # ソースコード
│   ├── main.rs                 # エントリーポイント（eframe 起動）
│   ├── app.rs                  # AppState・メインループ
│   ├── cli.rs                  # ファイル/フォルダパスのリポジトリルート解決(ドラッグ&ドロップ等から利用)
│   ├── config.rs               # 設定ファイルの読み書き
│   ├── git/                    # Git ロジックレイヤー
│   │   ├── mod.rs
│   │   ├── repository.rs       # GitRepository ラッパー
│   │   ├── commit.rs           # CommitInfo 取得・変換
│   │   └── diff.rs             # DiffFile / DiffHunk 取得・変換
│   └── ui/                     # UI レイヤー（egui パネル）
│       ├── mod.rs
│       ├── toolbar.rs          # ToolbarPanel
│       ├── commit_list.rs      # CommitListPanel
│       └── diff_panel.rs       # DiffPanel（ファイル一覧 + diff 表示）
├── tests/                      # 統合テスト
│   └── git_integration.rs      # テスト用リポジトリを使った統合テスト
├── docs/                       # プロジェクトドキュメント
│   ├── product-requirements.md
│   ├── functional-design.md
│   ├── architecture.md
│   ├── repository-structure.md （本ドキュメント）
│   ├── development-guidelines.md
│   └── glossary.md
├── .claude/                    # Claude Code 設定
├── .steering/                  # 作業単位ドキュメント
├── Cargo.toml                  # プロジェクト定義・依存関係
├── Cargo.lock                  # 依存関係ロックファイル（コミット必須）
├── .gitignore
├── CLAUDE.md                   # Claude Code プロジェクト設定
└── README.md                   # プロジェクト概要（将来作成）
```

## ディレクトリ詳細

### src/

#### src/main.rs

**役割**: アプリケーションのエントリーポイント。`eframe::run_native()` を呼び出して GUI を起動する。

**配置内容**:
- `fn main()` のみ
- eframe の設定（ウィンドウタイトル・サイズ）

**依存関係**:
- 依存可能: `crate::app`
- 依存禁止: `crate::git`、`crate::ui`（間接的に使うが直接 import しない）

#### src/app.rs

**役割**: `AppState` 構造体の定義と、egui の `eframe::App` トレイト実装。毎フレーム `update()` が呼ばれ、UI 描画と状態更新を行う。

**配置内容**:
- `AppState` 構造体
- `impl eframe::App for AppState` の `update()` 実装
- UI パネルへの描画委譲
- Git ロジック呼び出し（コミット読み込み・diff 読み込み）

**依存関係**:
- 依存可能: `crate::git`、`crate::ui`、`crate::config`
- 依存禁止: なし（最上位の統合レイヤー）

#### src/cli.rs

**役割**: ドラッグ&ドロップ等で渡されたファイル/フォルダの絶対パスを、Git リポジトリルートと(ファイル指定時のみ)そのファイルの相対パスに解決する。

**配置内容**:
- `CliTarget` 構造体
- `resolve_target(raw_path: &Path) -> Result<CliTarget, GitError>`

**依存関係**:
- 依存可能: `crate::git`（`GitError` を利用）
- 依存禁止: `crate::ui`

#### src/config.rs

**役割**: `AppConfig` の定義と、TOML ファイルへの読み書き。

**配置内容**:
- `AppConfig` 構造体（serde derive）
- `load_config() -> AppConfig`
- `save_config(config: &AppConfig)`

**依存関係**:
- 依存可能: `serde`、`toml`、標準ライブラリ
- 依存禁止: `crate::git`、`crate::ui`

#### src/git/

**役割**: Git 操作の純粋なロジック層。`git2` クレートへのアクセスをここに集約する。

**配置ファイル**:

| ファイル | 役割 |
|---------|------|
| `mod.rs` | モジュール公開・共通の型定義（`GitError` 等） |
| `repository.rs` | `GitRepository` ラッパー構造体 |
| `commit.rs` | `CommitInfo` の取得・変換ロジック |
| `diff.rs` | `DiffFile`・`DiffHunk`・`DiffLine` の取得・変換ロジック |

**命名規則**:
- ファイル名: `snake_case.rs`
- 構造体: `PascalCase`
- 関数: `snake_case`、動詞で始める（`load_commits`, `parse_diff_line`）

**依存関係**:
- 依存可能: `git2`、`chrono`、標準ライブラリ
- 依存禁止: `crate::ui`、`crate::config`、`egui`

#### src/ui/

**役割**: egui を使った各パネルの描画ロジック。状態の読み書きは `AppState` 経由で行う。

**配置ファイル**:

| ファイル | 役割 |
|---------|------|
| `mod.rs` | モジュール公開 |
| `toolbar.rs` | ToolbarPanel（パス表示・開くボタン） |
| `commit_list.rs` | CommitListPanel（コミット一覧・スクロール） |
| `diff_panel.rs` | DiffPanel（ファイル一覧 + diff 表示） |

**命名規則**:
- ファイル名: `snake_case.rs`
- 関数名: `show_<panel_name>(ui: &mut egui::Ui, state: &mut AppState)`

**依存関係**:
- 依存可能: `egui`、`crate::app`（AppState の型のみ）
- 依存禁止: `crate::git`（直接は呼ばない。`AppState` に格納済みのデータを参照）

### tests/

#### tests/git_integration.rs

**役割**: テスト用一時リポジトリを構築して `src/git/` の統合テストを行う。

**構造**:
```
tests/
└── git_integration.rs    # #[test] 関数を直接記述
```

ユニットテスト（`#[cfg(test)] mod tests`）は各ソースファイルの末尾に配置する（Rust の慣習に従う）。

## ファイル配置規則

### ソースファイル

| ファイル種別 | 配置先 | 命名規則 | 例 |
|------------|--------|---------|-----|
| ロジック・モジュール | `src/git/` | `snake_case.rs` | `commit.rs` |
| UI パネル | `src/ui/` | `snake_case.rs` | `commit_list.rs` |
| 設定・永続化 | `src/` 直下 | `snake_case.rs` | `config.rs` |

### テストファイル

| テスト種別 | 配置先 | 命名規則 |
|-----------|--------|---------|
| ユニットテスト | 各ソースファイル末尾（`#[cfg(test)]`） | ソースと同ファイル |
| 統合テスト | `tests/` | `<対象>.rs` |

### 設定ファイル（ルート）

| ファイル | 用途 |
|---------|------|
| `Cargo.toml` | クレート定義・依存関係 |
| `Cargo.lock` | 依存関係のロック（**必ずコミットする**） |
| `.gitignore` | Git 除外設定 |
| `CLAUDE.md` | Claude Code プロジェクト設定 |

## 命名規則

### ディレクトリ名
- `snake_case`（Rust の慣習）
- 例: `src/git/`、`src/ui/`

### ファイル名
- Rust ソースファイル: `snake_case.rs`
- ドキュメント: `kebab-case.md`

### Rust コード内の命名
| 対象 | 規則 | 例 |
|------|------|-----|
| 構造体・列挙型 | `PascalCase` | `CommitInfo`, `DiffLineKind` |
| 関数・メソッド | `snake_case` | `load_commits`, `show_toolbar` |
| 定数 | `UPPER_SNAKE_CASE` | `MAX_DIFF_SIZE_BYTES` |
| モジュール | `snake_case` | `git`, `commit` |
| トレイト | `PascalCase` | `eframe::App` |

## 依存関係のルール

### レイヤー間の依存（許可される方向）

```
src/ui/
    ↓ （AppState を通じて参照）
src/app.rs
    ↓
src/git/    src/config.rs
```

**禁止される依存**:
- `src/git/` → `src/ui/` (❌)
- `src/git/` → `src/config.rs` (❌)
- `src/ui/` → `src/git/` (❌ 直接呼び出しは禁止。AppState 経由のみ)

### 循環依存の禁止
Rust のコンパイラが検出するが、設計上も循環しないようにレイヤーを一方向に保つ。

## スケーリング戦略

### Post-MVP 機能追加時の方針

**ステージング・コミット機能を追加する場合**:
```
src/git/
├── mod.rs
├── repository.rs
├── commit.rs
├── diff.rs
└── write.rs          ← 新規追加（書き込み系操作を分離）
```

**新しい UI パネル追加**:
```
src/ui/
├── mod.rs
├── toolbar.rs
├── commit_list.rs
├── diff_panel.rs
└── staging_panel.rs  ← 新規追加
```

### ファイルサイズの目安
- 1ファイル 300 行以下を推奨
- 300 行超えたらモジュール分割を検討

## 特殊ディレクトリ

### .steering/ （ステアリングファイル）

**役割**: 作業単位の設計・タスクリスト・振り返りを記録

```
.steering/
└── [YYYYMMDD]-[作業名]/
    ├── requirements.md
    ├── design.md
    ├── tasklist.md
    └── retrospective.md
```

### .claude/ （Claude Code 設定）

```
.claude/
├── commands/       # スラッシュコマンド定義
├── skills/         # スキル定義
├── agents/         # サブエージェント定義
├── hooks/          # フック設定
├── settings.json   # 共有設定
└── README.md       # カタログ
```

## 除外設定（.gitignore）

```gitignore
# Rust ビルド成果物
/target/

# 環境固有設定（共有しない）
.claude/settings.local.json

# OS
.DS_Store
Thumbs.db

# ステアリング（作業単位のドキュメント。コミット対象外）
# ※ Git 運用ルールにより feature ブランチでコミットするため、
#   retrospective.md は除外しない
```
