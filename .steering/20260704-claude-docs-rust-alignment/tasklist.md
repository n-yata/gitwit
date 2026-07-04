# タスクリスト

## 🚨 タスク完全完了の原則

**このファイルの全タスクが完了するまで作業を継続すること**

### 必須ルール
- **全てのタスクを`[x]`にすること**
- 「時間の都合により別タスクとして実施予定」は禁止
- 「実装が複雑すぎるため後回し」は禁止
- 未完了タスク(`[ ]`)を残したまま作業を終了しない

### タスクスキップが許可される唯一のケース
以下の技術的理由に該当する場合のみスキップ可能:
- 実装方針の変更により、機能自体が不要になった
- アーキテクチャ変更により、別の実装方法に置き換わった
- 依存関係の変更により、タスクが実行不可能になった

---

## フェーズ1: development-guidelines skill の Rust 化

- [x] `guides/implementation.md` を Rust 仕様に全面書き換え
  - [x] 型定義セクション(構造体・enum・型注釈の原則)を Rust の例に置き換え
  - [x] 命名規則セクション(変数・関数・型・定数・ファイル名)を Rust の例に置き換え
  - [x] 関数設計セクション(単一責務・関数の長さ・パラメータ)を Rust の例に置き換え
  - [x] エラーハンドリングセクション(`Result`/`?`演算子・カスタムエラー型)を Rust の例に置き換え
  - [x] コメント規約セクション(`///` ドキュメントコメント・WHYコメント)を Rust の例に置き換え
  - [x] 非同期処理セクションを Rust 相当(存在しなければ削除、または `std::thread`/コールバック等プロジェクト実態に合わせた記述に置換)に置き換え
  - [x] セキュリティ・パフォーマンスセクションを Rust の例に置き換え
  - [x] テストコードセクション(`#[cfg(test)] mod tests`)に置き換え
  - [x] リファクタリングセクション(マジックナンバー排除・関数抽出)を Rust の例に置き換え
  - [x] チェックリスト内の「TSDocコメント」等 TypeScript 固有表現を Rust 表現(`///` ドキュメントコメント等)に置き換え
- [x] `guides/process.md` を Rust ツールチェーン前提に書き換え
  - [x] テスト戦略の Given-When-Then コード例を Rust(`#[test]`)の例に置き換え
  - [x] カバレッジ設定例(`jest.config.js`)を Rust 相当(`cargo llvm-cov` 等)の例に置き換え、またはプロジェクトの実態(`docs/development-guidelines.md` のカバレッジ目標)に合わせた記述に統一
  - [x] コードレビュー例中のコードスニペットを Rust の例に置き換え
  - [x] 品質自動化セクション(Lint/フォーマット/型チェック/テスト/ビルド)のツール名を `cargo fmt`/`cargo clippy`/`cargo test`/`cargo build` に置き換え
  - [x] CI/CD例(GitHub Actions)を Rust toolchain セットアップベースに書き換え
  - [x] Pre-commitフック例(Husky)を Rust 向け(例: `cargo fmt --check`/`cargo clippy` を実行するフックスクリプト)に置き換え
  - [x] 「この構成を選んだ理由」の説明文を Rust エコシステム向けに書き換え
- [x] `SKILL.md` のクイックリファレンス記述(「TypeScript/JavaScript規約」等)を Rust 向けの表現に更新
- [x] `template.md` の見出し(「TypeScript/JavaScript」等)を更新(実際にはコード例本文もRustへ全面差し替え)

## フェーズ2: その他 skill 内の例示記述の Rust 寄り置き換え

- [x] `architecture-design/guide.md` の技術スタック例(Node.js/TypeScript/npm)を Rust/cargo/git2/egui 等の例に差し替え
- [x] `architecture-design/template.md` の技術スタックテーブル例を同様に差し替え
- [x] `repository-structure/guide.md` 内のファイル命名・ディレクトリ構成例(`TaskService.ts` 等)を Rust のモジュール構成例(`.rs`、`snake_case`、`mod.rs`)に差し替え
- [x] `repository-structure/template.md` 内の同様の例を Rust のモジュール構成例に差し替え
- [x] `glossary/guide.md`・`template.md` 内の「TypeScript」用語定義例・`src/types/Task.ts` 等のパス例を Rust プロジェクトに即した用語例・パス例に差し替え
- [x] `functional-design/guide.md` 内のデータモデル定義例(TypeScript interface)を Rust の `struct` 定義例に差し替え(「ステップ3: データモデル定義」節は既にRust struct例になっていたため、「まとめ」節に残っていた「TypeScript型定義」の一文のみ修正)
- [x] `steering/templates/tasklist.md` のビルド確認チェック項目(`npm test`等)を `cargo test`/`cargo clippy -- -D warnings`/`cargo fmt --check`/`cargo build` に置き換え

## フェーズ3: 品質チェックと修正

- [x] フェーズ1・2で編集した全対象ファイルに対し、Grep で `TypeScript|JavaScript|npm |Node\.js|ESLint|Vitest|Jest|Prettier|Husky` を検索し、意図せず残った記述がないか確認
  - [x] `.claude/skills/development-guidelines/` 配下(1件ヒットしたが、Rust enumをTypeScriptのユニオン型と比較する説明的コメントであり意図的な記述のため対応不要)
  - [x] `.claude/skills/architecture-design/` 配下(該当なし)
  - [x] `.claude/skills/repository-structure/` 配下(該当なし)
  - [x] `.claude/skills/glossary/guide.md`(該当なし)
  - [x] `.claude/skills/functional-design/guide.md`(該当なし)
  - [x] `.claude/skills/steering/templates/tasklist.md`(該当なし)
- [x] `CLAUDE.md`・`docs/development-guidelines.md`・`.claude/README.md` に差分がないことを確認(`git diff --stat` で無変更を確認済み)
- [x] 書き換え後の Rust コード例が `docs/development-guidelines.md` の規約(命名規則・`unwrap`禁止・エラー型パターン等)と矛盾しないか目視レビュー(`implementation.md`内の`unwrap()`使用箇所2件を確認。1件は意図的な「❌悪い例」、もう1件はテストコード内でどちらも規約通り)

## フェーズ4: ドキュメント更新

- [x] `.claude/README.md` の更新要否を確認(各skillの一覧・一行説明は変更していないため更新不要と判断)
- [x] 実装後の振り返りを記録(別ファイル `retrospective.md` に記録 → モード3)

---

> **振り返りについて**: 実装後の振り返りはこのファイルではなく、同じディレクトリの
> `retrospective.md` に記録する(テンプレート: `.claude/skills/steering/templates/retrospective.md`)。
> 全タスクが `[x]` になったことを確認してから作成すること。
