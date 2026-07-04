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

## フェーズ1: Git ロジック層(src/git/diff.rs, repository.rs)

- [x] diff.rsのtree比較ロジックを共通ヘルパーに切り出す
  - [x] delta→DiffFile変換ロジックを private ヘルパー関数(例: `diff_tree_to_files(repo, old_tree: Option<Tree>, new_tree: &Tree)`)に抽出する
  - [x] patch→DiffHunk変換ロジックを private ヘルパー関数(例: `diff_tree_to_hunks(repo, old_tree: Option<Tree>, new_tree: &Tree, file_path: &str)`)に抽出する
  - [x] 既存の `load_diff_files(repo, oid_str)` を上記ヘルパー呼び出しに置き換え、戻り値・シグネチャは変えず既存動作を維持する
  - [x] 既存の `load_diff_hunks(repo, oid_str, file_path)` を上記ヘルパー呼び出しに置き換え、既存動作を維持する
- [x] 2コミット間差分の関数を追加する
  - [x] `load_diff_files_between(repo, base_oid_str, target_oid_str) -> Result<Vec<DiffFile>, GitError>` を実装する(base/targetそれぞれのcommitからtreeを取得しヘルパーへ渡す)
  - [x] `load_diff_hunks_between(repo, base_oid_str, target_oid_str, file_path) -> Result<Vec<DiffHunk>, GitError>` を実装する
- [x] repository.rsにpublicラッパーを追加する
  - [x] `GitRepository::load_diff_files_between(&self, base_oid_str: &str, target_oid_str: &str)` を追加
  - [x] `GitRepository::load_diff_hunks_between(&self, base_oid_str: &str, target_oid_str: &str, file_path: &str)` を追加
- [x] diff.rsに統合テストを追加する
  - [x] 一時ディレクトリに実リポジトリを作成し3コミット(C1→C2→C3)を積むテストヘルパーを用意する(commit.rsの既存テストパターンを踏襲)
  - [x] `load_diff_files_between(C1, C3)` がC1→C3間の累積差分ファイル一覧を返すことを検証するテストを書く
  - [x] `load_diff_hunks_between(C1, C3, path)` が該当ファイルの差分行を返すことを検証するテストを書く

## フェーズ2: ViewModel層(src/app.rs)

- [x] `AppState.selected_commit: Option<usize>` を `selected_commits: Vec<usize>` に置き換える
  - [x] フィールド定義を変更する
  - [x] `AppState::new` の初期値を `Vec::new()` に変更する
  - [x] `load_repo` 内のリセット処理(`self.state.selected_commit = None`)を `self.state.selected_commits.clear()` に変更する
- [x] `load_diff_files` を選択件数に応じて分岐させる
  - [x] `selected_commits.len() == 0`: 何もせず早期return(現行動作維持)
  - [x] `selected_commits.len() == 1`: 従来通り `repo.load_diff_files(&oid)` を呼ぶ
  - [x] `selected_commits.len() == 2`: `commits[idx].time` を比較し古い方をbase・新しい方をtargetとして `repo.load_diff_files_between(&base_oid, &target_oid)` を呼ぶ
- [x] `load_diff_hunks` も同様に選択件数で分岐させる(単一時は従来通り、2件時は`load_diff_hunks_between`)

## フェーズ3: UI層(src/ui/commit_list.rs, diff_panel.rs)

- [x] commit_list.rsでShiftキー押下を検知する処理を追加する(`ui.input(|i| i.modifiers.shift)`)
- [x] クリックハンドリングをスライディング選択ロジックに置き換える
  - [x] 通常クリック(Shiftなし): `state.selected_commits = vec![idx]`
  - [x] Shift+クリック かつ 選択が空: 通常クリックと同じ扱い(`vec![idx]`)にする
  - [x] Shift+クリック かつ 選択が1件以上: 既存の同idxを`retain`で除去してから`push`し、`len() > 2`なら`remove(0)`でスライディングさせる
  - [x] クリック後は既存同様 `needs_diff_load = true` と diff_files/selected_file/diff_hunksのクリアを行う
- [x] ハイライト表示を更新する
  - [x] `is_selected` 判定を `state.selected_commits.contains(&idx)` に変更する
  - [x] 2件選択時に単一選択時と区別できる新しい色定数(例: `COLOR_SELECTED_RANGE`)を追加し、選択件数に応じて出し分ける
- [x] diff_panel.rsの `selected_commit` 参照箇所を修正する
  - [x] `show_diff_panel` 内の `state.selected_commit.is_none()` を `state.selected_commits.is_empty()` に置き換える

## フェーズ4: 品質チェックと修正

- [x] すべてのテストが通ることを確認
  - [x] `cargo test`
- [x] リントエラーがないことを確認
  - [x] `cargo clippy -- -D warnings`
- [x] フォーマットが崩れていないことを確認
  - [x] `cargo fmt --check`(今回変更したファイルのみ対象。cli.rs/commit.rsに既存の未整形ドリフトがあるが本機能とは無関係のため対象外)
- [x] ビルドが成功することを確認
  - [x] `cargo build`

## フェーズ5: ドキュメント更新

- [x] `docs/functional-design.md` にマルチ選択・2コミット間diff表示の仕様を反映する必要があるか確認し、必要なら更新する
- [x] 実装後の振り返りを記録(別ファイル `retrospective.md` に記録 → モード3)

---

> **振り返りについて**: 実装後の振り返りはこのファイルではなく、同じディレクトリの
> `retrospective.md` に記録する（テンプレート: `.claude/skills/steering/templates/retrospective.md`）。
> 全タスクが `[x]` になったことを確認してから作成すること。
