# rnr

[`@antfu-collective/ni`](https://github.com/antfu-collective/ni) の `nr`（package.json の
script ランナー）を Rust で再実装する**個人練習プロジェクト**。

> Phase 0〜4 まで実装済み。`rnr <script>` / 引数なしの fuzzy 選択 / `rnr -` /
> volta・mise 解決が動き、npm 配布構造（optionalDependencies + bin shim）も
> 1ターゲット（darwin-arm64）で組んである。publish は未実施。

## なぜ作るか

実用ツールで本家を置き換えるためではなく、次の2つを体で覚えるため:

1. Rust の言語そのもの（所有権 / `Result` / `match` / 文字列の取り回し）に慣れる
2. **Rust 製 CLI を npm エコシステムへ配布する**実務パターン（oxlint / esbuild 方式）を一周組む

題材に `nr` を選んだ理由や設計判断は [PLAN.md](./PLAN.md) を参照。

## やること（`nr` の本質）

1. lockfile から package manager を検出（npm / pnpm / yarn / bun）
2. `package.json` の `scripts` を読む
3. `rnr <script>` → `<pm> run <script>` を exec（stdio 継承・exit code 透過）
4. `rnr`（引数なし）→ fuzzy な対話選択で script を選ぶ
5. `rnr -` → 直前に実行した script を再実行
6. volta / mise の解決（mise が active なら volta に横取りさせない）

ロードマップは Phase 0〜4 として [PLAN.md](./PLAN.md) に記載。

## インストール（ローカルで使う）

`cargo install` で release ビルドが `~/.cargo/bin/rnr` に入り、どこからでも使える:

```sh
git clone <repo-url> rnr
cd rnr
cargo install --path .
```

`~/.cargo/bin` が PATH に通っていれば、任意の Node プロジェクトで `rnr` が使える。

```sh
which rnr            # ~/.cargo/bin/rnr が出ればOK
cargo install --path . --force   # コード更新後に入れ直す
cargo uninstall rnr              # アンインストール
```

> これは自分のマシンで使うためのインストール方法。`pnpm add github:...` のような
> GitHub 直インストールは現状は不可（ルートに package.json が無く、npm パッケージは
> `npm/` 配下にあり、バイナリも未公開のため）。他人に配るなら下記の npm 配布が必要。

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

開発中に出た疑問（serde / anyhow / shim の仕組みなど）は [build-log.md](./build-log.md) に記録。

## npm 配布の構造（Phase 4）

esbuild / oxlint 方式の「optionalDependencies + bin shim」を1ターゲットで再現したもの。

```
npm/
├── rnr/                       # 本体パッケージ
│   ├── package.json           #   optionalDependencies で各プラットフォームを参照
│   └── bin/rnr.js             #   shim: require.resolve で実バイナリを解決して spawn
└── platforms/
    └── darwin-arm64/          # @rnr/darwin-arm64
        ├── package.json       #   os/cpu 指定（合致環境でだけ install される）
        └── rnr                #   release バイナリ（gitignore。下記で配置）
```

ローカルで結線して試す:

```sh
cargo build --release
cp target/release/rnr npm/platforms/darwin-arm64/rnr   # バイナリを配置
npm pack ./npm/platforms/darwin-arm64                  # → tーボール
npm pack ./npm/rnr                                     # → tーボール
# 消費側プロジェクトで両 tーボールを install すると node_modules/.bin/rnr が使える
```

実際の `npm i -g rnr` では、shim → Rust バイナリ → 各 PM の3段で stdio 継承・exit code 透過が通る。
publish はしていない（やるなら手動 1 回）。

## ライセンス

MIT
