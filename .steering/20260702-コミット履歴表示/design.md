# 設計: コミット履歴表示

## ディレクトリ構造（実装後）

```
git-client/
├── src/
│   ├── main.rs          # eframe::run_native() のみ
│   ├── app.rs           # AppState + eframe::App 実装
│   ├── config.rs        # AppConfig + TOML 読み書き
│   ├── git/
│   │   ├── mod.rs       # GitError + pub use
│   │   ├── repository.rs # GitRepository::open()
│   │   └── commit.rs    # load_commits() + format_relative_time()
│   └── ui/
│       ├── mod.rs
│       ├── toolbar.rs   # show_toolbar()
│       └── commit_list.rs # show_commit_list()
├── Cargo.toml
└── Cargo.lock
```

## 主要な設計判断

### ファイルダイアログなし（初回）
rfd クレートは Windows での動作確認が必要で導入コストが高い。
初回はテキスト入力フィールドでパスを直接入力する形式にする。
ユーザビリティより「動くこと」を優先。

### コミット上限は 1,000 件
パフォーマンス要件（3秒以内）を満たすために初期実装で上限を設ける。
仮想スクロールは次フェーズ。

### egui の即時モードとの整合
状態変化（コミット読み込み）は `update()` 内で行う。
`app.rs` の `update()` でフラグ (`needs_load`) を確認し、
Git 操作を呼び出してから state を更新する。

## データフロー

```
1. 起動時
   config.load() → last_repo_path があれば open_repo() → load_commits()

2. 「開く」ボタン押下
   path_input の文字列 → open_repo(path) → load_commits()
   → state.commits 更新 → egui 再描画

3. エラー時
   GitError → Display impl → state.error_message にセット → UI に表示
```
