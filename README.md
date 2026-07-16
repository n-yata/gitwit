# gitwit

Rust製の軽量なGit GUIクライアント(Windows専用)。コミット履歴一覧・diff確認をシンプルに行う。

## ダウンロード(ビルド済みexe)

[Releases](https://github.com/n-yata/gitwit/releases) から最新の `gitwit-vX.Y.Z-windows-x86_64.zip` をダウンロードして展開するだけで使える。

各Releaseには `.sha256` ファイルも添付している。改ざんされていないか確認する場合は以下を実行する:

```powershell
# ダウンロードしたzipと同じフォルダで実行
Get-FileHash .\gitwit-vX.Y.Z-windows-x86_64.zip -Algorithm SHA256
```

出力されたハッシュ値が、同梱の `.sha256` ファイルの内容と一致することを確認する。

exeはGitHub Actions(`.github/workflows/release.yml`)がタグpushをトリガーに、このリポジトリのソースコードから自動ビルドしたものであり、手元でのビルド物を直接コミットしたものではない。

## ソースからビルドする

```powershell
git clone https://github.com/n-yata/gitwit.git
cd gitwit
cargo build --release
```

ビルドされた実行ファイルは `target/release/gitwit.exe` に生成される。

## 開発

詳細な開発方針は `CLAUDE.md` および `docs/` 配下のドキュメントを参照。

```powershell
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```
