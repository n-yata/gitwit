# .claude カタログ

このディレクトリに置かれた **commands / skills / agents** の目次です。
「何があるか」を素早く見渡すための案内板であり、各項目の正本は
それぞれの `SKILL.md` / command の frontmatter `description` です（カタログはそれを要約します）。

> **メンテナンスのルール**: `.claude/` 配下に command / skill / agent を
> **追加・削除・リネームしたら、この README.md も必ず同じ変更で更新する**こと。
> 詳細はプロジェクトの `CLAUDE.md`「.claude カタログの維持」を参照。

---

## Commands（`/name` で明示的に起動するワークフロー）

ユーザーが意図的に `/name` とタイプして始めるワークフローのエントリーポイント。

| コマンド | 概要 | 起動 |
|---|---|---|
| **plan-feature** | 新機能の要求を planモードで対話的に練り、固まったら `.steering/[日付]-[タイトル]/requirements.md` を作成する。`/add-feature` の前段で、実装には進まず要求書の作成で停止する。 | `/plan-feature` |
| **add-feature** | `requirements.md` を起点に、設計（design.md / tasklist.md 生成）→ 実装ループ → 実装検証（implementation-validator）→ テスト → 振り返りまでを無停止で自動実行する。 | `/add-feature [機能名]` |
| **review-docs** | doc-reviewer サブエージェントを起動し、指定ドキュメントを完全性・明確性・一貫性・実装可能性・測定可能性の観点で詳細レビューする。 | `/review-docs [ドキュメントパス]` |
| **setup-project** | 初回セットアップ。`docs/ideas/` を入力に、永続ドキュメント6種（PRD・機能設計・アーキテクチャ・リポジトリ構造・開発ガイドライン・用語集）を対話的に作成する。 | `/setup-project` |

---

## Skills（description により自動ロード／`/name` でも起動）

状況に応じて自動的にロードされる知識・手順・テンプレート。`/name` での明示起動も可能。

### ドキュメント作成系（成果物名で命名。`docs/` 配下の永続ドキュメントを作る）

| skill | 概要 | 成果物 |
|---|---|---|
| **prd** | プロダクト要求定義書(PRD)を作成・更新する。プロダクトビジョン・ペルソナ・KPI・機能/非機能要件を定義する。 | `docs/product-requirements.md` |
| **functional-design** | 機能設計書を作成・更新する。PRD の要件を技術的にどう実現するかを設計する。 | `docs/functional-design.md` |
| **architecture-design** | アーキテクチャ設計書を作成・更新する。システム構造・技術選定（テクノロジースタック）を定義する。 | `docs/architecture.md` |
| **repository-structure** | リポジトリ構造定義書を作成・更新する。技術スタックを反映した具体的なディレクトリ構造を定義する。 | `docs/repository-structure.md` |
| **development-guidelines** | 開発ガイドラインを作成・更新する。コーディング規約・命名規則・Git運用・テスト戦略の参照元でもある。 | `docs/development-guidelines.md` |
| **glossary** | 用語集を作成・更新する。プロジェクト固有の用語・ユビキタス言語を体系的に定義する。 | `docs/glossary.md` |

### ワークフロー系（作業の進め方を支援する）

| skill | 概要 |
|---|---|
| **steering** | 作業単位の計画・実装・振り返りを `.steering/` に記録する。モード1（ステアリングファイル作成）／モード2（実装と tasklist.md の進捗管理）／モード3（振り返り retrospective.md の作成）を持つ。 |
| **grill-with-docs** | 永続ドキュメント作成の前段として、アイデアをインタビュー形式の壁打ちで掘り下げ、`docs/ideas/` に書き出す。固めた内容が PRD 等の入力になる。 |
| **archive-retrospectives** | `.steering/` 配下の振り返り（retrospective.md）を棚卸しし、`docs/` へ昇格すべき学びを承認制で反映したうえで、処理済みディレクトリを `.steering/archives/` へアーカイブする。 |

---

## Agents（サブエージェント）

独立したコンテキストで動作する専門エージェント。主に command から起動される。

| agent | 概要 | 主な起動元 | model |
|---|---|---|---|
| **doc-reviewer** | ドキュメントの品質を完全性・明確性・一貫性・実装可能性・測定可能性の5観点で評価し、優先度別に改善提案を行う。 | `/review-docs` | sonnet |
| **implementation-validator** | 実装コードをスペック準拠・コード品質・テストカバレッジ・セキュリティ・パフォーマンスの5観点で検証する。 | `/add-feature` | sonnet |

> ※ 設計・テスト・インフラ・セキュリティの専門チームメンバー
> （architecture-designer / test-engineer / devops-engineer / security-engineer）は
> ユーザーグローバル（`~/.claude/agents/`）に定義されており、本カタログの対象外。
