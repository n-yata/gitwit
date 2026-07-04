# 設計書

## アーキテクチャ概要

本作業はコード実装を伴わない、Markdown ドキュメント(`.claude/skills/` 配下の guide/template/SKILL.md)の書き換えのみを対象とする。既に Rust 仕様で正しく書かれている `docs/development-guidelines.md` を「正本(リファレンス)」とし、そこで採用されている Rust の規約・コード例のスタイルを、対象の skill ファイル群へ機械的に転写・整合させる。

```
docs/development-guidelines.md (Rust仕様・正本、変更しない)
        │ 参照して整合させる
        ▼
.claude/skills/development-guidelines/{guides/implementation.md, guides/process.md, SKILL.md, template.md}
.claude/skills/architecture-design/{guide.md, template.md}
.claude/skills/repository-structure/{guide.md, template.md}
.claude/skills/glossary/guide.md
.claude/skills/functional-design/guide.md
.claude/skills/steering/templates/tasklist.md
```

## コンポーネント設計

### 1. development-guidelines skill の書き換え

**責務**:
- 実装ガイド(命名規則・型定義→構造体/enum・エラーハンドリング・コメント・テスト)を Rust 仕様に書き換える
- プロセスガイド(テスト戦略・カバレッジ・品質自動化・CI)を Rust ツールチェーンベースに書き換える
- SKILL.md・template.md の見出し・説明文から「TypeScript/JavaScript」表記を排除する

**実装の要点**:
- `docs/development-guidelines.md` に既にある Rust の書き方(`GitError` enum、`Result<T, E>`、`#[cfg(test)]`、`cargo fmt`/`clippy` 等)をそのまま踏襲し、新規に矛盾するスタイルを作らない
- Git運用ルール(ブランチ戦略・コミットメッセージ・PRテンプレート等)は言語非依存のため、変更対象は「コード例」「ツール名」部分のみに限定する
- CI例(GitHub Actions)は `npm ci`/`setup-node` を `dtolnay/rust-toolchain` + `cargo build/test/fmt/clippy` 相当に置き換える

### 2. その他 skill の例示コード置き換え

**責務**:
- architecture-design の技術スタック例テーブルを Rust/cargo/git2/egui ベースに変更
- repository-structure のファイル命名・ディレクトリ構成例を `.rs`/`snake_case`/`mod.rs` ベースに変更
- glossary の「TypeScript」用語例・`.ts` パス例を Rust 用語例・`.rs` パス例に変更
- functional-design の TypeScript interface データモデル例を Rust `struct` 定義例に変更
- steering/templates/tasklist.md のビルド確認コマンドを `cargo` ベースに変更

**実装の要点**:
- これらは「書き方のサンプル」を提供するテンプレートであり、本プロジェクト(Rust/egui/git2)を題材にした具体例に差し替えることで、一貫性と実用性を両立する
- 構造(見出し・箇条書きの粒度)自体は変更せず、中身のコード例・技術名のみ差し替える

## データフロー

### 書き換え作業の流れ
```
1. 対象ファイルを Read で読み込み、TypeScript/JavaScript/Node.js/npm 固有の記述箇所を特定
2. docs/development-guidelines.md のRust表現を参照しながら、Edit で該当箇所を書き換え
3. 書き換え後、対象ファイルに TS/JS/npm 固有キーワードが残っていないか Grep で確認
```

## エラーハンドリング戦略

対象外(コード実装なしのため、本セクションは適用しない)。

## テスト戦略

コード変更を伴わないため自動テスト(`cargo test` 等)は対象外。代わりに以下で整合性を検証する:

- **静的確認**: 書き換え後の各対象ファイルに対して `Grep` で `TypeScript|JavaScript|npm |Node\.js|ESLint|Vitest|Jest|Prettier|\.tsx?\b` を検索し、ヒットしないことを確認する
- **内容レビュー**: 書き換えた Rust コード例が `docs/development-guidelines.md` の規約(命名規則・`unwrap` 禁止・エラー型等)と矛盾しないか目視で確認する

## 依存ライブラリ

なし(ドキュメントのみの変更)。

## ディレクトリ構造

変更されるファイル(新規作成なし、すべて既存ファイルの編集):

```
.claude/skills/development-guidelines/guides/implementation.md   (編集)
.claude/skills/development-guidelines/guides/process.md          (編集)
.claude/skills/development-guidelines/SKILL.md                   (編集)
.claude/skills/development-guidelines/template.md                (編集)
.claude/skills/architecture-design/guide.md                      (編集)
.claude/skills/architecture-design/template.md                   (編集)
.claude/skills/repository-structure/guide.md                     (編集)
.claude/skills/repository-structure/template.md                  (編集)
.claude/skills/glossary/guide.md                                 (編集)
.claude/skills/functional-design/guide.md                        (編集)
.claude/skills/steering/templates/tasklist.md                    (編集)
```

## 実装の順序

1. development-guidelines skill(guides/implementation.md → guides/process.md → SKILL.md → template.md)
2. architecture-design(guide.md → template.md)
3. repository-structure(guide.md → template.md)
4. glossary(guide.md)
5. functional-design(guide.md)
6. steering/templates/tasklist.md
7. 全対象ファイルへの Grep による残存チェック

## セキュリティ考慮事項

- ドキュメント内にシークレット・実URLを書かないこと(既存のCLAUDE.mdルールに準拠。今回はコード例のみなので該当リスクは低い)

## パフォーマンス考慮事項

該当なし(ドキュメントのみの変更)。

## 将来の拡張性

将来 `docs/development-guidelines.md` の規約(命名規則・エラー型・テスト方針等)が変わった場合は、本作業で書き換えた skill 側の guide/template も追従して更新する必要がある。両者が正本と参考資料の関係にあることを `SKILL.md` の記述で明示しておく。
