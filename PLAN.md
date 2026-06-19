# rnr — Rust 版 `nr`

[`@antfu-collective/ni`](https://github.com/antfu-collective/ni) の `nr`（package.json の script 実行）を
Rust で再実装する**個人練習プロジェクト**。

目的は実用ツールで本家を置き換えることではなく、

1. Rust の言語そのもの（所有権 / `Result` / `match` / 文字列の取り回し）に慣れる
2. **Rust製CLIを npm エコシステムへ配布する**実務パターン（oxlint / esbuild 方式）を一周組む

の2点を体で覚えること。

> なぜ `nr` か: 仕様が明確で規模が小さく（本家で ~1,700行）、入力（lockfile + args）と
> 出力（実行する argv）が決まっているので「期待コマンド == 実コマンド」のテストが書きやすい。
> 一方で計算は軽い（lockfile を読んで exec するだけ）グルーコードなので、Rust の言語税は
> 速度では回収しづらい——が、**練習ではその税金を払う過程自体が学びの本体**になる。

## `nr` の本質（移植する振る舞い）

1. **lockfile から package manager を検出**（npm / pnpm / yarn / bun）
2. **package.json の `scripts` を読む**
3. 引数あり → `<pm> run <script> [extra args]` を組んで **exec（stdio 継承・exit code 透過）**
4. 引数なし → **fuzzy な対話選択 UI** で script を選ぶ（`nr` で一番使う機能）
5. `nr -` → **直前に実行した script を再実行**（storage に保存した lastRunCommand）
6. 実行直前に **volta ラッパー**判定（ここに mise 改善を入れる / 下記）

後回しでよいもの: shell completion 生成、`-p`（monorepo workspace）、`runAgent: node`。

## 構成（2レイヤー）

```
rnr/
├── Cargo.toml
├── PLAN.md
├── src/
│   ├── main.rs       # 引数パース → dispatch
│   ├── detect.rs     # lockfile 検出 → Agent enum
│   ├── package.rs    # package.json の scripts を読む（serde）
│   ├── command.rs    # Agent + script → argv 組み立て（+ volta/mise）
│   ├── runner.rs     # std::process::Command で exec、exit code 透過
│   ├── storage.rs    # lastRunCommand の保存/読み込み
│   └── prompt.rs     # 対話 fuzzy 選択（Phase 2）
└── npm/              # 配布レイヤー（Phase 4）
    ├── rnr/          # 本体パッケージ（optionalDependencies + bin shim）
    └── platforms/    # @rnr/<os>-<arch> を CI で生成
```

### 依存（最小から）

| 用途 | crate | 入れる時期 |
| --- | --- | --- |
| エラー | `anyhow` | Phase 0（導入済み） |
| package.json パース | `serde` + `serde_json` | Phase 0（導入済み） |
| 子プロセス exec | `std::process::Command` | 標準（依存なし） |
| 引数パース | 最初は手書き `std::env::args` → 慣れたら `clap` | Phase 0 / 後で |
| 対話 fuzzy 選択 | `inquire`（fuzzy filter 内蔵の `Select`） | Phase 2 |

> 最初は依存を `anyhow` + `serde` だけに絞り、言語そのものに集中する。

## フェーズ

### Phase 0 — 動く最小 `rnr <script>`
- `Agent` enum（`Npm` / `Pnpm` / `Yarn` / `Bun`）
- lockfile 検出のみ: `pnpm-lock.yaml`→Pnpm, `yarn.lock`→Yarn, `bun.lockb`/`bun.lock`→Bun,
  `package-lock.json`→Npm、無ければ Npm にフォールバック
- `package.json` の `scripts` を serde で読む
- `rnr build` → `pnpm run build` を `Command::new(...).status()` で exec、exit code をそのまま返す
- **ゴール**: `rnr <既存 script>` が本物のように動く

### Phase 1 — `--dry-run`（テスト可能化）
- 実行せず「組み立てた argv」を print するフラグ（本家の `?` 相当）
- `tests/` に「lockfile + args → 期待 argv」のテストを書く
- **ゴール**: `cargo test` で移植の正しさを担保

### Phase 2 — 対話 fuzzy 選択 + `rnr -`（コア機能）
- `inquire` の `Select` で `key - description` 一覧を fuzzy 絞り込み → 実行
- **lastRunCommand を先頭にピン留め**（本家挙動）。ESC でキャンセル
- `storage.rs`: 選んだ script を一時ファイル（`~/.cache` 等）に保存、`rnr -` で読む
- **ゴール**: 日常使いできる UX（スクショの候補リストを再現）

### Phase 3 — volta / mise（本家にない自分の理想挙動）
- 実行直前に: `volta` が PATH にあり、**mise が active でなければ** `volta run <cmd>` でラップ
- mise active 判定は環境変数（`MISE_SHELL` / `__MISE_DIFF` など）で
- **ゴール**: volta が入っていても mise を使っていれば volta に横取りされない

### Phase 4 — npm 配布（oxlint / esbuild 方式）
- GitHub Actions で各ターゲットをビルド: mac arm64/x64・linux x64/arm64（gnu + musl）・win x64(msvc)
  - linux クロス / musl は `cargo-zigbuild` か `cross`
- 各バイナリを `@rnr/<os>-<arch>` として publish（各 package.json に `os` / `cpu` を指定 → 非該当は自動スキップ）
- 本体 `rnr` パッケージが全部を `optionalDependencies`、`bin` の shim が該当バイナリを
  `require.resolve` して exec
- **ゴール**: `npm i -g rnr` で実機に該当バイナリ 1 個だけ落ちる、を体験

## 参考

- 移植元の仕様: [antfu-collective/ni `src/commands/nr.ts`](https://github.com/antfu-collective/ni/blob/main/src/commands/nr.ts)
- PM 検出ロジックの正解表: [package-manager-detector](https://github.com/antfu-collective/package-manager-detector)
- npm 配布の教科書: [oxc-project/oxc](https://github.com/oxc-project/oxc) の oxlint パッケージ + `.github/workflows`
