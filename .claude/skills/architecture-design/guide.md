# アーキテクチャ設計ガイド

## 基本原則

### 1. 技術選定には理由を明記

**悪い例**:
```
- Rust
- egui
```

**良い例**:
```
- Rust (最新安定版)
  - メモリ安全性を保証しつつネイティブ並みの実行速度を実現できる
  - 所有権システムによりGCなしでリソース管理でき、デスクトップアプリに適する
  - cargoエコシステムが充実しており、必要なクレートの入手が容易

- egui 0.x
  - 純Rust製の即時モードGUIで、追加ランタイム(Electron等)が不要で軽量
  - クロスプラットフォーム対応で、将来的な他OS展開の余地がある
  - シンプルなAPIで、VSCodeライクな軽量UIを素早く構築できる

- git2 0.x (libgit2バインディング)
  - プロセス起動(git コマンド呼び出し)なしで高速にGit操作できる
  - Rustの型システムを通してlibgit2のAPIを安全に扱える
```

### 2. レイヤー分離の原則

各レイヤーの責務を明確にし、依存関係を一方向に保ちます:

```
UI → Service → Data (OK)
UI ← Service (NG)
UI → Data (NG)
```

### 3. 測定可能な要件

すべてのパフォーマンス要件は測定可能な形で記述します。

## レイヤードアーキテクチャの設計

### 各レイヤーの責務

**UIレイヤー(egui ウィジェット)**:
```rust
// 責務: ユーザー入力の受付と表示
impl App {
    // OK: AppState 経由でGitロジックを呼び出す
    fn on_filter_changed(&mut self, path: PathBuf) {
        self.state.set_file_filter(path);
        self.state.needs_load = true;
    }

    // NG: git2 のAPIを直接UIから呼び出す
    fn on_filter_changed_bad(&mut self, path: PathBuf) {
        let _ = git2::Repository::open(&path); // ❌ UIがGitロジックに直接依存
    }
}
```

**ViewModel / AppState レイヤー**:
```rust
// 責務: UI状態の保持とGitロジックの呼び出し
impl AppState {
    fn load_commits(&mut self) -> Result<(), GitError> {
        let commits = self.repository.load_commits(1000)?;
        self.commits = commits;
        self.needs_load = false;
        Ok(())
    }
}
```

**Git ロジックレイヤー(git2クレート)**:
```rust
// 責務: Gitリポジトリ操作の実装
impl GitRepository {
    fn load_commits(&self, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        let mut revwalk = self.inner.revwalk().map_err(GitError::Git2)?;
        revwalk.push_head().map_err(GitError::Git2)?;
        // ...
        Ok(Vec::new())
    }
}
```

## パフォーマンス要件の設定

### 具体的な数値目標

```
コミット履歴の表示: 200ms以内(平均的なPC環境で)
└─ 測定方法: std::time::Instant でリポジトリオープンから一覧描画まで計測
└─ 測定環境: CPU Core i5相当、メモリ8GB、SSD

コミット一覧の描画: 60fps を維持(egui の再描画コスト込み)
└─ 測定方法: 1000コミットのテストリポジトリで計測
└─ 許容範囲: 100件で50ms、1000件で200ms、10000件で1秒
```

## セキュリティ設計

### データ保護の3原則

1. **最小権限の原則**
```bash
# ファイルパーミッション(設定ファイル等を扱う場合)
chmod 600 ~/.gitwit/config.toml  # 所有者のみ読み書き
```

2. **入力検証**
```rust
fn validate_repo_path(path: &Path) -> Result<(), GitError> {
    if !path.exists() {
        return Err(GitError::NotARepository(path.to_path_buf()));
    }
    if git2::Repository::open(path).is_err() {
        return Err(GitError::NotARepository(path.to_path_buf()));
    }
    Ok(())
}
```

3. **機密情報の管理**
```bash
# 認証情報が必要な場合は環境変数で管理し、コード内にハードコードしない
export GITWIT_CREDENTIAL_HELPER="xxxxx"
```

## スケーラビリティ設計

### データ増加への対応

**想定データ量**: [例: 10,000件のコミット履歴]

**対策**:
- コミット一覧の遅延読み込み(ページネーション)
- 大きすぎるdiffのスキップ(`MAX_DIFF_SIZE_BYTES`)
- `git2::Revwalk` の絞り込みによる走査範囲の最適化

```rust
// 大きすぎるファイルの diff をスキップする例
fn build_diff_line(file_size: u64, content: &str) -> Result<DiffLine, GitError> {
    if file_size > MAX_DIFF_SIZE_BYTES {
        return Err(GitError::LargeFile(file_size));
    }
    Ok(DiffLine::from_content(content))
}
```

## 依存関係管理

### バージョン管理方針

```toml
# Cargo.toml
[dependencies]
git2 = "0.19"      # マイナーバージョンアップは自動(セマンティックバージョニングに従う)
egui = "=0.29.1"   # 破壊的変更のリスクがある場合は完全固定
eframe = "0.29"
chrono = "0.4"
```

**方針**:
- 安定版は `Cargo.toml` のデフォルト指定(キャレット互換、マイナーバージョンまで許可)
- 破壊的変更のリスクがある場合(UIフレームワーク等)は完全固定(`=`)
- `Cargo.lock` をコミットし、ビルドの再現性を担保する

## チェックリスト

- [ ] すべての技術選定に理由が記載されている
- [ ] レイヤードアーキテクチャが明確に定義されている
- [ ] パフォーマンス要件が測定可能である
- [ ] セキュリティ考慮事項が記載されている
- [ ] スケーラビリティが考慮されている
- [ ] バックアップ戦略が定義されている
- [ ] 依存関係管理のポリシーが明確である
- [ ] テスト戦略が定義されている