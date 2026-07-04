# タスクリスト

## 🚨 タスク完全完了の原則

**このファイルの全タスクが完了するまで作業を継続すること**

### 必須ルール
- **全てのタスクを`[x]`にすること**
- 「時間の都合により別タスクとして実施予定」は禁止
- 「実装が複雑すぎるため後回し」は禁止
- 未完了タスク（`[ ]`）を残したまま作業を終了しない

### タスクスキップが許可される唯一のケース
以下の技術的理由に該当する場合のみスキップ可能:
- 実装方針の変更により、機能自体が不要になった
- アーキテクチャ変更により、別の実装方法に置き換わった
- 依存関係の変更により、タスクが実行不可能になった

スキップ時は必ず理由を明記:
```markdown
- [x] ~~タスク名~~（実装方針変更により不要: 具体的な技術的理由）
```

---

## フェーズ1: ドロップされたパスの抽出ロジック

- [x] `src/app.rs` に `first_dropped_path(files: &[egui::DroppedFile]) -> Option<PathBuf>` を実装する
  - [x] `path: Some(..)` を持つ先頭要素のパスを返す
  - [x] 空配列・`path: None` のみの場合は `None` を返す
- [x] `first_dropped_path` のユニットテストを追加する
  - [x] 空配列 → `None`
  - [x] 単一要素(パスあり) → そのパス
  - [x] 複数要素、先頭にパスあり → 先頭のパス(2件目以降は無視)

## フェーズ2: 状態反映ロジック(`App::apply_dropped_path`)

- [x] `App` に `apply_dropped_path(&mut self, path: PathBuf)` メソッドを実装する
  - [x] `cli::resolve_target(&path)` の `Ok` 時: `state.path_input` / `state.file_filter` を更新し `state.needs_load = true`、`state.error_message = None` にする
  - [x] `Err` 時: `state.error_message` にエラーメッセージをセットし、他の状態(`path_input`/`file_filter`/`commits`/`repo_path`)は変更しない
- [x] `apply_dropped_path` のユニットテスト(または同等の検証)を追加する
  - [x] Gitリポジトリ配下のフォルダパスを渡すと `file_filter` に相対パスがセットされ `needs_load` が `true` になる
  - [x] リポジトリのルートパスを渡すと `file_filter` が `None` になる
  - [x] リポジトリ外のパスを渡すと `error_message` がセットされ、他の状態(既存の `path_input` 等)が変化しない

## フェーズ3: `App::update` への組み込み

- [x] `eframe::App::update` の冒頭(既存の `needs_load` 等のチェックより前)で `ctx.input(|i| i.raw.dropped_files.clone())` を読み取り、`first_dropped_path` → `apply_dropped_path` を呼び出す処理を追加する
- [x] 既存の `needs_load` / `needs_diff_load` / `needs_file_load` の消化処理が同一フレーム内でそのまま機能することを確認する(コードレビューで確認、既存ロジックは無改修)

## フェーズ4: 手動動作確認

- [x] `cargo run` でアプリを起動し、パニックせず正常にウィンドウが立ち上がることを確認する(自動実行での確認範囲。バックグラウンド起動→プロセス生存確認→正常終了を確認済み)
- [x] ~~フォルダをドラッグ&ドロップして、そのフォルダ配下のコミット履歴のみが表示されることを確認する~~（自動化不可: 実際のマウスによるOSネイティブD&D操作はCLIエージェントのツールセットでは再現できない。design.md記載の通りこの機能は元々「自動E2E化は行わず手動確認でカバーする」方針。ロジックは `apply_dropped_path_sets_file_filter_for_subfolder` 単体テストで検証済み。シャビによる実機での最終確認を推奨）
- [x] ~~単一ファイルをドラッグ&ドロップして、そのファイルの変更を含むコミットのみが表示されることを確認する~~（同上の理由により自動化不可。ロジックは `resolve_target` の既存テスト(`resolve_target_with_file_computes_relative_path`)で検証済み）
- [x] ~~現在開いているリポジトリと異なるリポジトリ配下のパスをドロップして、リポジトリが切り替わり履歴が表示されることを確認する~~（同上の理由により自動化不可。`apply_dropped_path` が `path_input`/`file_filter` を書き換え `needs_load=true` にすることは単体テストで検証済みで、既存の `load_repo()` がそのまま別リポジトリを開き直す設計であることをコードレビューで確認済み）
- [x] ~~複数ファイル/フォルダを同時にドロップして、先頭の1件のみが反映されることを確認する~~（同上の理由により自動化不可。`first_dropped_path_returns_the_first_entry_and_ignores_the_rest` 単体テストで検証済み）
- [x] ~~Gitリポジトリ外のパスをドロップして、既存のエラーモーダルが表示され、現在の表示状態が変化しないことを確認する~~（同上の理由により自動化不可。`apply_dropped_path_sets_error_and_keeps_state_for_path_outside_repo` 単体テストで検証済み）
- [x] ~~トップバーの「履歴フィルタ: {path}」表示と「✕」クリアボタンが、ドロップ経由の絞り込みでも既存同様に機能することを確認する~~（同上の理由により自動化不可。`toolbar.rs` の表示ロジックは `state.file_filter` のみを参照しており、CLI起動経由・ドロップ経由で分岐しない実装であることをコードレビューで確認済み）

## フェーズ5: 品質チェックと修正

- [x] すべてのテストが通ることを確認
  - [x] `cargo test`(実装検証(implementation-validator)の指摘を反映後、18 passed; 0 failed)
- [x] implementation-validator によるレビュー指摘の反映
  - [x] 非公開関数のドキュメントコメントを `///` から `//` に変更(development-guidelines準拠)
  - [x] `first_dropped_path` の `[None, Some(path)]` ケースのテストを追加
  - [x] worktree内に残っていた手動確認用ログファイル(`run_stdout.log`/`run_stderr.log`)を削除
- [x] リントエラーがないことを確認
  - [x] `cargo clippy --all-targets -- -D warnings`(警告なし)
- [x] ~~ビルドが成功することを確認(`cargo build --release`)~~（本機能の変更に起因しない既存の環境問題によりスキップ: `cargo build --release` は syn クレートのコンパイルで `bound modifier ?can only be applied to Sized` エラーが発生し失敗するが、未変更の `master` ブランチで同一コマンドを実行しても全く同じ場所で同じエラーが再現することを確認済み(=このマシンのRustツールチェイン環境固有の問題であり、本PRの変更とは無関係)。`cargo build`(devプロファイル)は問題なく成功しており、devプロファイルの動作確認は完了している）

## フェーズ6: ドキュメント更新

- [x] 実装内容がプロジェクトのアーキテクチャ・機能設計に影響するか確認し、必要であれば `docs/functional-design.md` を更新する(AppStateへのfile_filterフィールド追記とUC-3追加を実施)
- [x] 実装後の振り返りを記録（別ファイル `retrospective.md` に記録 → モード3）

---

> **振り返りについて**: 実装後の振り返りはこのファイルではなく、同じディレクトリの
> `retrospective.md` に記録する（テンプレート: `.claude/skills/steering/templates/retrospective.md`）。
> 全タスクが `[x]` になったことを確認してから作成すること。
