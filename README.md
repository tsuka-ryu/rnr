# rnr

[`@antfu-collective/ni`](https://github.com/antfu-collective/ni) を Rust で再実装した CLI。
lockfile から package manager（npm / pnpm / yarn / bun）を検出し、PM 差を吸収して実行する。

- **`rnr`**（`nr` 相当）— package.json の script ランナー。`<pm> run <script>`
- **`rni`**（`ni` 相当）— 依存インストール。`rni` / `rni <pkg>` / `-D` / `-g`
- **`rnci`**（`nci` 相当）— frozen install（CI 用の再現インストール）

実体は1個のバイナリで、呼ばれた名前（`argv[0]`）で振る舞いを変える **busybox 方式**。

> 個人練習プロジェクト。作る目的・設計判断・ロードマップは [PLAN.md](./PLAN.md)（nr 系）と
> [PLAN-NI.md](./PLAN-NI.md)（ni 系）、開発中に出た疑問のメモは
> [build-log.md](./build-log.md) を参照。

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

`rni` / `rnci` は同じバイナリへの symlink として配る（busybox 方式）:

```sh
ln -s ~/.cargo/bin/rnr ~/.cargo/bin/rni
ln -s ~/.cargo/bin/rnr ~/.cargo/bin/rnci
```

## 使い方

### `rnr` — script 実行（`nr` 相当）

```sh
rnr                  # 引数なし → fuzzy 選択 UI（直前に実行した script を先頭にピン留め）
rnr <script>         # <pm> run <script> を実行（exit code は透過）
rnr <script> -- --x  # 追加引数を渡す
rnr -                # 直前に実行した script を再実行
rnr --dry-run <script>   # 実行せず、組み立てた argv を表示
```

選択 UI（引数なし）の操作: ↑↓ で移動、文字入力で fuzzy 絞り込み、Enter で決定、ESC でキャンセル。

### `rni` / `rnci` — 依存インストール（`ni` 相当）

```sh
rni                  # lockfile に従って全依存をインストール
rni <pkg>            # 依存を追加
rni -D <pkg>         # devDependencies に追加
rni -g <pkg>         # global インストール
rnci                 # frozen install（lockfile を凍結したまま再現）
rni --dry-run <pkg>  # 実行せず、組み立てた argv を表示
```

検出した PM ごとに argv を組み替える（yarn は v1 基準）:

| 操作 | npm | pnpm | yarn | bun |
| --- | --- | --- | --- | --- |
| `rni` | `npm i` | `pnpm i` | `yarn install` | `bun install` |
| `rni <pkg>` | `npm i {p}` | `pnpm add {p}` | `yarn add {p}` | `bun add {p}` |
| `rni -D <pkg>` | `npm i -D {p}` | `pnpm add -D {p}` | `yarn add -D {p}` | `bun add -d {p}` |
| `rni -g <pkg>` | `npm i -g {p}` | `pnpm add -g {p}` | `yarn global add {p}` | `bun add -g {p}` |
| `rnci` | `npm ci` | `pnpm i --frozen-lockfile` | `yarn install --frozen-lockfile` | `bun install --frozen-lockfile` |

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