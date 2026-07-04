# 設計書

## アーキテクチャ概要

既存のレイヤー構成(UI → ViewModel/AppState → Git ロジック)を維持したまま、以下3層それぞれに変更を加える。

```
UI (commit_list.rs)      : Shift+クリック検知 → 選択状態(Vec)更新
        ↓
ViewModel (app.rs)        : 選択件数に応じて「単一コミット差分」/「2コミット間差分」を分岐しロード
        ↓
Git ロジック (diff.rs)    : old_tree/new_tree の取得元だけを切り替える共通ヘルパーで両ケースを実現
```

diff_panel.rs(ファイル一覧・diffビューの描画)は diff_files/diff_hunks を描画するだけの汎用実装のため、**無改修**で流用する。

## コンポーネント設計

### 1. AppState の選択状態(src/app.rs)

**責務**:
- 選択中コミットを「クリック順」で保持する
- 最大2件のスライディング選択を表現する

**実装の要点**:
- `selected_commit: Option<usize>` を `selected_commits: Vec<usize>` に置き換える(クリック順を保持。時系列順ではない)。
- 要素は常に0〜2件。3件目のShift+クリックで `remove(0)`(最初にクリックした方)を削除してから追加する。
- 同じコミットへの重複追加を避けるため、追加前に既存の同idxを`retain`で除去してから`push`する。
- 単一/複数の判定はこのVecの長さで行う(`len() == 1` → 従来の親コミット比較、`len() == 2` → 2コミット間比較)。

### 2. commit_list.rs のクリックハンドリング

**責務**:
- Shiftキー押下状態を検知し、通常クリックとShift+クリックで挙動を分岐する

**実装の要点**:
- `ui.input(|i| i.modifiers.shift)` でShift押下を取得(eguiの標準API)。
- 通常クリック: `state.selected_commits = vec![idx]`(現行動作を維持)。
- Shift+クリック かつ 選択が空でない場合のみスライディング選択ロジックを適用。選択が空の状態でのShift+クリックは通常クリックと同じ扱いにする(単一選択開始)。
- ハイライトは `state.selected_commits.contains(&idx)` で判定。2件選択時は単一選択時と区別できるよう、新しい色定数 `COLOR_SELECTED_RANGE`(既存 `COLOR_SELECTED` とは別の色味)を選択件数に応じて出し分ける。

### 3. diff.rs のtree比較ロジック共通化

**責務**:
- 単一コミット(親との差分)と2コミット間差分の両方を、共通のtree比較ヘルパーで実現する

**実装の要点**:
- 既存の `load_diff_files`/`load_diff_hunks` 内にある「delta→DiffFile変換」「patch→DiffHunk変換」ロジックを private ヘルパー(`diff_tree_to_files(repo, old_tree: Option<Tree>, new_tree: &Tree) -> Result<Vec<DiffFile>, GitError>` / `diff_tree_to_hunks(repo, old_tree: Option<Tree>, new_tree: &Tree, file_path: &str) -> Result<Vec<DiffHunk>, GitError>`)に切り出す。
- 既存の `load_diff_files(repo, oid_str)` は `old_tree = commit.parent(0).tree()`, `new_tree = commit.tree()` を渡すだけの薄いラッパーに変更(戻り値・シグネチャは維持し既存呼び出し元に影響なし)。
- 新規に `load_diff_files_between(repo, base_oid_str, target_oid_str)` / `load_diff_hunks_between(repo, base_oid_str, target_oid_str, file_path)` を追加し、`old_tree = base_commit.tree()`, `new_tree = target_commit.tree()` を渡す。
- `src/git/repository.rs` の `GitRepository` に上記2つのpublicラッパーメソッドを追加する。

### 4. app.rs のロード分岐

**責務**:
- 選択件数に応じて単一/2コミット間比較を分岐し、既存の`needs_diff_load`/`needs_file_load`パターンに乗せる

**実装の要点**:
- `load_diff_files`: `state.selected_commits` の長さで分岐。base/target決定は純粋関数 `resolve_diff_oids(commits, selected)` に切り出し、`commits` 配列のindex(revwalkのTIMEソート順、0が最新)を履歴順の正として比較する(同一秒コミットでも`time`同値による非決定性が出ないようにするため、`commit.time`ではなくindexで比較する)。
  - 1件: 従来通り `repo.load_diff_files(&oid)`
  - 2件: indexが大きい方(古い)をbase、小さい方(新しい)をtargetとして `repo.load_diff_files_between(&base_oid, &target_oid)`
  - 0件: 何もしない(現行の早期returnを維持)
- `load_diff_hunks` も同様に分岐する。
- `diff_panel.rs` の未選択判定 `state.selected_commit.is_none()` は `state.selected_commits.is_empty()` に置き換える。

## データフロー

### 2コミット選択→diff表示
```
1. ユーザーがコミットAを通常クリック → selected_commits = [A]、needs_diff_load = true
2. ユーザーがコミットBをShift+クリック → selected_commits = [A, B]、needs_diff_load = true
3. app.update() が needs_diff_load を検知 → load_diff_files() 実行
4. selected_commits.len() == 2 のため、commits[A].time / commits[B].time を比較し base/target を決定
5. repo.load_diff_files_between(base_oid, target_oid) を呼び diff_files を更新
6. diff_panel.rs は従来通り diff_files を描画(無改修)
```

## エラーハンドリング戦略

新規エラー型は追加しない。既存の `GitError` / `Result<_, GitError>` パターンをそのまま踏襲する(oid parse失敗・commit/tree取得失敗はすべて `GitError::Git2` に集約済み)。

## テスト戦略

### ユニットテスト
- (対象なし。commit_list.rsのクリックロジックはegui Ui依存で単体分離が難しいため、選択状態遷移のロジック自体をdiff.rs同様に小さな純粋関数として切り出せる場合のみ追加。無理に分離せず、実装時に自然な形になれば追加する)

### 統合テスト(src/git/diff.rs内 `#[cfg(test)] mod tests`)
- 一時ディレクトリに実リポジトリを作成し、3つのコミットを積む(C1→C2→C3)。
- `load_diff_files_between(C1, C3)` が C1→C3 間の累積差分ファイル一覧を返すこと。
- `load_diff_hunks_between(C1, C3, path)` が該当ファイルの差分行を返すこと。
- 引数の順序を入れ替えて(target→baseで呼ぶケースがないことを設計上保証しているため)呼び出し側(app.rs)でのbase/target決定ロジックが正しく古い方をbaseにしていることを検証する軽いテストも検討(app.rs側かdiff.rs側、実装しやすい層で追加)。

## 依存ライブラリ

追加なし(既存の git2, egui, eframe のみで実現可能)。

## ディレクトリ構造

既存ファイルの変更のみ。新規ファイル追加なし。

```
src/
  app.rs                 (変更: selected_commit → selected_commits, ロード分岐)
  ui/
    commit_list.rs        (変更: Shift+クリック検知、スライディング選択、ハイライト色分岐)
    diff_panel.rs          (変更: selected_commit参照箇所をselected_commitsに合わせて修正)
  git/
    diff.rs                (変更: tree比較ヘルパー共通化 + between系関数追加 + テスト追加)
    repository.rs           (変更: between系publicメソッド追加)
```

## 実装の順序

1. `src/git/diff.rs`: 共通ヘルパー切り出し + `_between`系関数追加 + テスト
2. `src/git/repository.rs`: `_between`系のpublicラッパー追加
3. `src/app.rs`: `selected_commit` → `selected_commits` へ置き換え、ロード分岐ロジック実装
4. `src/ui/commit_list.rs`: Shift+クリック検知・スライディング選択・ハイライト色分岐
5. `src/ui/diff_panel.rs`: `selected_commit` 参照箇所の置き換え
6. 全体ビルド・テスト・lint確認

## セキュリティ考慮事項

- 新規に外部入力(ユーザー指定パス・コマンド実行等)を扱う変更はない。oid文字列は既存のコミット一覧(信頼できるgit2取得結果)由来のみで、外部からの直接入力を新たに受け付けない。

## パフォーマンス考慮事項

- 2コミット間比較は既存の単一コミット比較と同じ`diff_tree_to_tree`呼び出し1回分のコストで、計算量的な悪化はない。

## 将来の拡張性

- 本フェーズでは選択上限2件・隣接比較のみに限定する。将来的に3件以上の範囲選択や合算diffを追加する場合は、`selected_commits: Vec<usize>` の上限チェック部分とbase/target決定ロジック(現状は`min/max by time`)を拡張する形で対応できる設計にしている。
