# rnr

[`@antfu-collective/ni`](https://github.com/antfu-collective/ni) の `nr`（package.json の
script ランナー）を Rust で再実装した CLI。lockfile から package manager（npm / pnpm /
yarn / bun）を検出して `<pm> run <script>` を実行する。

> 個人練習プロジェクト。作る目的・設計判断・ロードマップは [PLAN.md](./PLAN.md)、
> 開発中に出た疑問のメモは [build-log.md](./build-log.md) を参照。

## インストール

`cargo install` で `~/.cargo/bin/rnr` に入り、任意の Node プロジェクトで使える:

```sh
git clone <repo-url> rnr
cd rnr
cargo install --path .
```

```sh
which rnr                         # ~/.cargo/bin/rnr が出ればOK
cargo install --path . --force    # コード更新後に入れ直す
cargo uninstall rnr               # アンインストール
```

## 使い方

```sh
rnr                  # 引数なし → fuzzy 選択 UI（直前に実行した script を先頭にピン留め）
rnr <script>         # <pm> run <script> を実行（exit code は透過）
rnr <script> -- --x  # 追加引数を渡す
rnr -                # 直前に実行した script を再実行
rnr --dry-run <script>   # 実行せず、組み立てた argv を表示
```

選択 UI（引数なし）の操作: ↑↓ で移動、文字入力で fuzzy 絞り込み、Enter で決定、ESC でキャンセル。

## 開発

```sh
cargo run -- <script>   # 実行
cargo test              # テスト
cargo build --release   # release ビルド（target/release/rnr）
```

`fixtures/` に各 package manager の lockfile を置いたお試し用プロジェクトがある。
詳細は [fixtures/README.md](./fixtures/README.md)。

## ライセンス

MIT