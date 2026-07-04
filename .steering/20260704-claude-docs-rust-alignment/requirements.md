# 要求内容

## 概要

`.claude/` 配下の skill ガイド・テンプレートに残っている TypeScript/JavaScript/Node.js 前提のコード例・記述を、本プロジェクトの技術スタック(Rust)に即した内容へ書き換える。`CLAUDE.md` および `docs/development-guidelines.md` は既に Rust 仕様で問題ないため対象外。

## 背景

`docs/development-guidelines.md`(実際にチームが参照する開発ガイドライン)は Rust の命名規則・`cargo fmt`/`clippy`・`Result<T, E>` によるエラーハンドリング等、正しく Rust 仕様で書かれている。

しかし `development-guidelines` skill 自身が参照資料として持つ `guides/implementation.md`・`guides/process.md` は、型定義・命名規則・エラーハンドリング・テスト・CI/CD の例が **まるごと TypeScript/JavaScript(ESLint, Vitest, Jest, Husky, npm 等)** で書かれている。`SKILL.md` のクイックリファレンスも「TypeScript/JavaScript規約」と明記しており、Rust プロジェクトである本リポジトリの実態と乖離している。

`architecture-design`・`repository-structure`・`glossary`・`functional-design` の各 skill にも、Node.js/TypeScript を前提にした技術選定サンプルやファイル命名例(`TaskService.ts` 等)が残っている。`steering/templates/tasklist.md` のビルド確認チェック項目も `npm test` 等 npm 前提になっている。

このスキル群は今後 `docs/development-guidelines.md` の再生成や新規ドキュメント作成時の参考資料として使われるため、内容が言語ミスマッチのまま放置されると、将来スキルを実行した際に誤って TypeScript 前提の規約を混入させるリスクがある。

## 実装対象の機能

### 1. development-guidelines skill の Rust 化

- `.claude/skills/development-guidelines/guides/implementation.md` を Rust 仕様に全面書き換える(型定義→構造体/enum、命名規則、エラーハンドリング→`Result`/`?`演算子、コメント規約、テストコード→`#[cfg(test)]` 等)。内容は `docs/development-guidelines.md` の既存記述と整合させる
- `.claude/skills/development-guidelines/guides/process.md` のうち、テスト戦略のコード例(Given-When-Then)・カバレッジ設定例(`jest.config.js`)・品質自動化セクション(ESLint/Prettier/tsc/Vitest/Husky/GitHub Actions の npm ワークフロー)を Rust の等価物(`cargo fmt`/`cargo clippy`/`cargo test`、GitHub Actions での Rust toolchain セットアップ等)に置き換える。Git運用ルールなど言語非依存のセクションはそのまま維持する
- `.claude/skills/development-guidelines/SKILL.md` のクイックリファレンス記述(「TypeScript/JavaScript規約」等)を Rust 向けの表現に更新する
- `.claude/skills/development-guidelines/template.md` の見出し(「TypeScript/JavaScript」等)を更新する

### 2. その他 skill 内の例示記述の Rust 寄り置き換え

- `.claude/skills/architecture-design/guide.md`・`template.md` の技術スタック例(Node.js/TypeScript/npm)を、Rust/cargo/git2/egui 等の例に差し替える
- `.claude/skills/repository-structure/guide.md`・`template.md` 内のファイル命名・ディレクトリ構成例(`TaskService.ts` 等)を、Rust のモジュール構成例(`.rs` ファイル、`snake_case`、`mod.rs` 等)に差し替える
- `.claude/skills/glossary/guide.md` 内の用語例(「TypeScript」の用語定義、`src/types/Task.ts` 等のパス例)を、Rust プロジェクトに即した用語例・パス例に差し替える
- `.claude/skills/functional-design/guide.md` 内のデータモデル定義例(TypeScript interface)を、Rust の構造体(`struct`)定義例に差し替える
- `.claude/skills/steering/templates/tasklist.md` のビルド確認チェック項目(`npm test`/`npm run lint`/`npm run typecheck`/`npm run build`)を `cargo test`/`cargo clippy -- -D warnings`/`cargo fmt --check`/`cargo build` に置き換える

## 受け入れ条件

### development-guidelines skill の Rust 化

- [ ] `guides/implementation.md` に TypeScript/JavaScript 固有のコード例が残っていない。すべて Rust のコード例に置き換わっている
- [ ] `guides/process.md` のテスト・カバレッジ・品質自動化セクションが Rust ツールチェーン(`cargo fmt`/`clippy`/`test`)を前提にした内容になっている
- [ ] `SKILL.md`・`template.md` の見出し・説明文に「TypeScript/JavaScript」という表現が残っていない

### その他 skill の例示記述

- [ ] `architecture-design`・`repository-structure`・`glossary`・`functional-design` の guide/template から、Node.js/TypeScript/npm 固有の具体例(`.ts` ファイル名等)が Rust 相当の例に置き換わっている
- [ ] `steering/templates/tasklist.md` のビルド確認チェック項目が `cargo` コマンドベースになっている

### 既存正本ドキュメントの非破壊

- [ ] `CLAUDE.md`・`docs/development-guidelines.md`・`.claude/README.md` の内容は変更しない(既に Rust 仕様で正しいため)

## 成功指標

- 定性的: `.claude/` 配下のどの skill を実行しても、Rust プロジェクトの実態と矛盾する言語前提のガイダンスが提示されない状態にする
- 定量的な目標は特に設定しない

## スコープ外

以下はこのフェーズでは実装しません:

- `docs/` 配下の正式ドキュメント(PRD・機能設計書・アーキテクチャ設計書等)の内容変更(既に Rust 仕様で記述済みのため対象外)
- skill/agent/command の新規追加・削除・構成変更(あくまで既存ファイル内の言語依存記述の是正が対象)
- `.claude/skills/prd`・`grill-with-docs`・`archive-retrospectives` 等、言語依存のコード例を含まない skill の変更

## 参照ドキュメント

- `docs/development-guidelines.md` - 開発ガイドライン(Rust仕様の正本。書き換え後の内容の整合基準とする)
- `docs/architecture.md` - アーキテクチャ設計書
- `.claude/README.md` - `.claude` カタログ

## 未決事項

なし(対話で全て確定済み)
