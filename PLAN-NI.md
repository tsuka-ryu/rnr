# rnr — `ni`（インストール系）を足す

既存の rnr（`nr` = script 実行）に、[`@antfu-collective/ni`](https://github.com/antfu-collective/ni)
の **`ni`（依存インストール系）** を追加するためのロードマップ。[PLAN.md](./PLAN.md) の続編。

目的は本家の置き換えではなく、`nr` で作った土台（検出・実行・volta/mise）の上に
**「PM 差を畳む解決テーブル」と「`argv[0]` dispatch」** を一周組んで理解すること。

## 進め方（PLAN.md と同じ前提）

- **実装は本人が手で書く。** Claude は実装コードを書かず、**ステップ単位の仕様だけ**を出す。
- 細切りで進める（1 ステップ仕様 → 本人が書く → レビュー → 次）。Phase をまとめて渡さない。
- Rust の文法ヒントは不要。指示は「関数のシグネチャ・振る舞い・本家との差分・エッジケース」に絞る。
- 各ステップは `cargo build` が通ることを確認してから次へ。
- 見積もり: `nr` の Phase 0+1 と同程度 ≒ 1〜2h（fuzzy UI が不要なぶん軽い）。

## `nr` との本質的な違い（ここが追加作業の核心）

`nr` は全 PM で `<bin> run <script>` と**一様**だった。
`ni` は **操作ごとに PM で argv が変わる**。本家でいう `AGENTS` マップ相当の
**解決テーブル**を持つことが、追加作業のほぼ全て。

| 操作 | npm | pnpm | yarn | bun |
| --- | --- | --- | --- | --- |
| 全部入れる（bare `ni`） | `npm i` | `pnpm i` | `yarn install` | `bun install` |
| 依存追加 `ni <pkg>` | `npm i {pkg}` | `pnpm add {pkg}` | `yarn add {pkg}` | `bun add {pkg}` |
| dev 追加 `-D` | `npm i -D {pkg}` | `pnpm add -D {pkg}` | `yarn add -D {pkg}` | `bun add -d {pkg}` |
| global `-g` | `npm i -g {pkg}` | `pnpm add -g {pkg}` | `yarn global add {pkg}` | `bun add -g {pkg}` |
| frozen（`nci` 相当） | `npm ci` | `pnpm i --frozen-lockfile` | `yarn install --frozen-lockfile` | `bun install --frozen-lockfile` |

> 正解表の出どころ: 本家 [`src/parse.ts`](https://github.com/antfu-collective/ni/blob/main/src/parse.ts) の `parseNi`。
> yarn は v1 を基準にする（`yarn add` / `yarn global add`）。Classic/Berry 差は後回し。

## 既存資産の再利用（ここは触らない）

| モジュール | `ni` でどうなる |
| --- | --- |
| [src/detect.rs](./src/detect.rs) | **そのまま**。同じ検出結果を使う |
| [src/runner.rs](./src/runner.rs) | **そのまま**。exec + exit code 透過は共通 |
| `wrap_volta` / `cmd_exists` / `mise_active`（[src/command.rs](./src/command.rs)） | **そのまま**。ni でも volta/mise 判定は同じ |
| [src/package.rs](./src/package.rs) | ほぼ不要（ni は scripts を読まない）。検出だけ流用 |
| [src/storage.rs](./src/storage.rs) | **使わない**（直前コマンド記憶は nr 専用） |
| [src/prompt.rs](./src/prompt.rs) | 任意。未インストール PM の auto-install 確認を出すなら crossterm を流用 |

新規で書くのは実質「引数 → 意図 → argv」の 1 ラインだけ。その後は既存の
`wrap_volta` → `runner::run` に**そのまま乗る**。

## 一番大きい設計判断：dispatch 方式

本家 ni は `ni` `nr` `nun` `nci` `nlx` … を**別名のシンボリックリンク**で配り、
`argv[0]`（呼ばれた名前）で振る舞いを変える（busybox 方式）。rnr に足すなら：

- **busybox 方式（推奨）** — バイナリ 1 個。`main.rs` 冒頭で `argv[0]` のファイル名を見て
  `rni` / `rnr` に分岐。配布時に名前違いの shim を複数置く。本家に忠実で、PLAN.md Phase 4 の
  「npm 配布の構造を学ぶ」目的とも噛み合う。
- `[[bin]]` 複数 — Cargo.toml に別バイナリ定義。単純だがバイナリが増える。

→ **busybox 方式で進める。**

## 構成（差分）

```
src/
├── main.rs       dispatch を追加: argv[0] のファイル名で ni / nr を振り分け
├── cli.rs (新)   ni 側の引数解釈: -D / -g / --frozen / -P / bare ni → 意図
├── install.rs(新) InstallSpec enum と (Agent, spec) → argv の解決テーブル（作業の本体）
├── command.rs    wrap_volta / cmd_exists / mise_active はそのまま共有。run 系は維持
├── detect.rs     変更なし
├── runner.rs     変更なし
└── package.rs    変更なし（ni からは使わない）
```

### 導入する型・関数（仕様のみ。シグネチャは目安）

- `enum InstallSpec { All, Frozen, Add { packages: Vec<String>, dev: bool, global: bool } }`
  — ni の引数を畳んだ「意図」。
- `fn parse_ni(args: &[String]) -> InstallSpec`
  — `-D`/`--save-dev` → `dev`、`-g` → `global`、`--frozen` → `Frozen`、
    package 名なし → `All`、それ以外 → `Add`。`nr` の `split_first` より一段複雑。
- `fn build_install_args(agent: Agent, spec: &InstallSpec) -> Vec<String>`
  — 上の表を `match (agent, spec)` で実装（`nr` の `build_run_args` の ni 版）。純粋関数。
- dispatch 後は `parse_ni` → `build_install_args` → `wrap_volta` → `runner::run` の一直線。

## フェーズ

### Phase N0 — `rni`（bare = 全部入れる）だけ
- `argv[0]` dispatch の骨格（`rni` という名前で呼ばれたら ni 経路へ）
- `InstallSpec::All` のみ。`detect → build_install_args(All) → wrap_volta → run`
- ローカルでの呼び分けは `ln -s` か `cargo run --bin` 相当で確認
- **ゴール**: `rni` が引数なしで「検出した PM の install」を本物のように実行する

### Phase N1 — `ni <pkg>` / `-D` / `-g`（依存追加）+ `--dry-run`
- `parse_ni` で `Add { packages, dev, global }` を解釈
- `build_install_args` に add / -D / -g の分岐を追加（上の表）
- `nr` と同じ `--dry-run` を ni 経路にも通し、`tests/` に
  「(agent, args) → 期待 argv」の表テストを書く
- **ゴール**: 主要な追加パターンが `cargo test` で担保される

### Phase N2 — `frozen`（`rnci` 相当）
- `--frozen` フラグ or `rnci` という名前での呼び出しを `InstallSpec::Frozen` に
- `build_install_args` に frozen 行を追加（npm のみ `ci`、他は `--frozen-lockfile`）
- **ゴール**: CI 用途の frozen install が通る

### Phase N3（任意）— auto-install 確認プロンプト
- 検出した PM が PATH に無いとき、`prompt.rs`（crossterm）で確認を出す
- 本家挙動だが学習上は後回しでよい

### 配布（PLAN.md Phase 4 の延長）
- busybox 方式なのでバイナリは 1 個のまま。`npm/rnr/bin/` に `rni.js` / `rnci.js` の
  shim を増やし、同じ実バイナリを違う `argv[0]` で spawn する構造を一周組む。
- フルマトリクスはやらない（PLAN.md と同じ方針）。

## 後回し / やらないもの
- yarn Berry（v2+）の `yarn add` 差分、`-P`（production）の厳密化
- `nun`（uninstall）/ `nup`（upgrade）/ `nlx`（execute）/ `na`（agent passthrough）
  — 同じテーブルに行を足すだけなので、ni が一周できてから
- monorepo `-r` / workspace 指定、shell completion

## 参考
- 移植元: [antfu-collective/ni `src/parse.ts`](https://github.com/antfu-collective/ni/blob/main/src/parse.ts)（`parseNi` と `AGENTS`/`COMMANDS` 表）
- PM 検出の正解表: [package-manager-detector](https://github.com/antfu-collective/package-manager-detector)
