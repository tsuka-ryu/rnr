# build-log — rnr 開発中に出た質問メモ

実装を進める中で疑問に思って質問したことの記録。

## Phase 0

### Q. lib.rs は必要？
`tests/`（Phase 1 の統合テスト）は **ライブラリ crate の公開 API しか触れない**。
binary だけ（main.rs に `mod`）だと tests から呼べないので、今のうちに
`src/lib.rs`（モジュール宣言だけ）+ `src/main.rs`（薄い殻）の2構成にした。
1 つの crate に lib.rs と main.rs があると Cargo が「ライブラリ rnr」+「バイナリ rnr」を自動で作る。Rust CLI の定番構成。

### Q. serde ってなに？
**ser**ialize / **de**serialize の略。Rust の構造体 ⇔ JSON/YAML/TOML を相互変換するライブラリ。
- `serde` … 変換の仕組み本体。`#[derive(Deserialize)]` で変換コードをマクロが自動生成。
- `serde_json` … serde を JSON 専用に実装したもの。`serde_json::from_str` が「JSON文字列 → 構造体」を実行。
- `#[serde(...)]` は serde への指示書き（属性）。`default` = キーが無ければデフォルト値、`rename` = JSONキー名とフィールド名を変える、など。

### Q. serde の使い方はどこを見ればわかる？
- 公式ガイド: https://serde.rs/ （特に field-attrs.html / container-attrs.html）
- API: https://docs.rs/serde_json/ , https://docs.rs/serde/
- ローカル: `cargo doc --open`（実際に使ってるバージョンのドキュメントが開く ← 一番確実）

### Q. anyhow ってなに？
Rust のエラーハンドリングを楽にするライブラリ（CLI/バイナリ向けの定番）。
種類の違うエラーを全部 `anyhow::Error` 1個に詰めることで、エラー型を自分で決めなくてよくなる。
- `anyhow::Result<T>` = `Result<T, anyhow::Error>` の略記
- `?` … 失敗時に関数を抜けてエラーを返す。異なる型のエラーも anyhow が自動変換
- `.with_context(|| "...")` … エラーに人間向け説明を1枚かぶせる（`anyhow::Context` トレイト）
- 対になる thiserror はライブラリ向け（呼び出し側がエラー種類で分岐したいとき）。rnr は CLI なので anyhow だけで足りる。

### Q. runner.rs の意味がわからない
組み立て済みの argv（例 `["pnpm","run","hello"]`）を別プロセスとして起動し、終了コードを返す関数。

```rust
let status = Command::new(&args[0]).args(&args[1..]).status()?;
Ok(status.code().unwrap_or(1))
```

- `Command::new(&args[0])` … 起動するコマンド名（"pnpm"）。Command は「子プロセスの設定書（ビルダー）」で、new した時点ではまだ起動しない。
- `.args(&args[1..])` … コマンド名以降の引数（["run","hello"]）を設定に足す。`[1..]` は index 1 以降のスライス。まだ起動しない。
- `.status()?` … ここで初めて子プロセスが起動。
  - stdio を親から継承する → 子の出力がそのまま端末に流れる（nr の挙動）。対比: `.output()` は出力を横取りして変数に溜める（画面に出ない）。
  - 子が終わるまで待つ（ブロック）。
  - 戻り値は `Result<ExitStatus, io::Error>`。失敗するのは「コマンドが見つからない」等の起動自体の失敗。`exit 7` で終わるのはエラーではない（起動は成功）。`?` で起動失敗だけ main に伝播。
- `status.code().unwrap_or(1)` … 終了コードを Option<i32> で取得。シグナルで殺されると None になるので、その場合は慣例で 1。
- これを main が `std::process::exit(code)` でそのまま rnr の終了コードにする = exit code 透過。

## Phase 4

### Q. bin shim ってなに？
npm パッケージが「コマンド」を提供するときの入口になる小さな実行ファイル。
package.json の `"bin": { "rnr": "bin/rnr.js" }` で、`npm i -g` 時に npm が PATH に `rnr` を貼り、それが `bin/rnr.js` を指す。ユーザーが `rnr build` と打つ → この JS が node で実行される。これが shim。

shim = 「隙間に挟む詰め物」。本体の前に挟まって橋渡しだけする薄い層。
今回は ユーザーの `rnr` と OS/arch ごとに別物の Rust バイナリ の間に挟まる:

```
rnr build → bin/rnr.js (shim) → require.resolve('@rnr/darwin-arm64/rnr') → 実バイナリを exec（exit code 透過）
```

なぜ必要か: npm パッケージは全 OS/arch 共通の1つで配るが、Rust バイナリは環境ごとに別物。
そこで 本体 rnr は JS shim だけ持ち、各バイナリは @rnr/<os>-<arch> に分けて optionalDependencies で
「合致する環境の1個だけ」入れる。shim が実行時に process.platform/process.arch から
パッケージ名を決め、入ってるバイナリを探して起動する。esbuild/oxlint/swc 等の定番パターン。

やってること自体は rnr の runner.rs と同じ（exec して exit code 透過）。
「正しいバイナリを探す」部分が付くだけ。

### Q. 実際はどうやって実行されるの？（npm 配布版の実行フロー）

**インストール時 `npm i -g rnr`:**
1. npm が本体 rnr の package.json を読む
2. optionalDependencies の @rnr/<os>-<arch> を見て、各候補の os/cpu を今のマシンと照合
3. 合致した1個だけインストール（他は optional なのでスキップ）→ node_modules が肥大化しない
4. 配置: node_modules/rnr/bin/rnr.js（shim） と node_modules/@rnr/darwin-arm64/rnr（実バイナリ）
5. bin 指定により npm が PATH に `rnr` コマンド（shim を指す）を作る

**実行時 `rnr build`（exec が2段）:**
```
rnr build
 → shell が PATH の rnr = shim(bin/rnr.js) を実行
 → shim: process.platform+arch="darwin-arm64" → require.resolve("@rnr/darwin-arm64/rnr") で実パス解決
 → ② spawnSync(実パス, ["build"], {stdio:'inherit'})
 → Rust バイナリ起動: detect → scripts 読む → "pnpm run build" 組み立て(mise active なら volta ラップ無し)
 → ③ Command::new("pnpm").args(["run","build"]).status()
 → pnpm run build が走る
```

**exit code は逆順に全段透過:** pnpm が 7 → Rust runner が exit(7) → shim の spawnSync result.status=7 → shim が exit(7) → shell の $?=7。stdio も全段 inherit なので出力もそのまま。

**cargo install 版との違い:**
- cargo install: shim 無し。~/.cargo/bin/rnr がいきなり Rust バイナリ。exec 1段。自分用。
- npm 配布: shim 経由で1段増える（node 起動で数十ms）。代わりに node さえあれば誰でも npm i で入る・OS/arch 自動選択。esbuild 等も同方式。

### Q. shim の仕組みのドキュメントはどこ？
単一の公式仕様は無い。npm の os/cpu/optionalDependencies/bin を組み合わせたコミュニティの定番パターン。
- 部品の公式 doc: npm package.json の os / cpu / optionalDependencies / bin
- 実装例（実質の教科書）: esbuild（元祖、install.js に詳しいコメント）、oxc/oxlint（PLAN の参考元、npm/ 構成）、Biome（公式 doc で明文化）、swc
- napi-rs は .node を require する N-API アドオン方式で、rnr の「単体バイナリを spawn」方式とは別物（混同注意）

## Phase 5 — プロンプト表示を本家 nr に寄せる

本家 nr のスクリプト選択 UI（`@posva/prompts` の autocomplete）と比べて、rnr（`inquire`）の見た目が
劣る点を調べて寄せた。本家 `src/commands/nr.ts` は候補を `title: key` / `description: command` の
2 フィールドで渡し、`limitText(description, terminalColumns - 15)` で切り詰めている。

### Q. 長いコマンドが折り返して見にくい。どう直す？
本家は `limitText(command, terminalColumns - 15)` で **固定 15 列ぶんを引いた幅に切り詰めて末尾に `…`** を付け、
1 行に収めている。rnr も同じく `terminal_columns()`（crossterm で取得、非 TTY 時は 80）から
固定 `RESERVED_COLUMNS = 15` を引いた幅で `limit_text` して折り返しを防ぐようにした（prompt.rs）。
最初は key 長から都度計算する実装にしたが、本家どおり固定値に合わせた。

### Q. スクロール上下記号（`^`/`v`）がわかりにくい。`↑`/`↓` にしたい
これは `inquire` の制約ではなく **設定で変えられる**。`RenderConfig` の `scroll_up_prefix` /
`scroll_down_prefix` を上書きすればよい。ベースは `RenderConfig::default()`（`NO_COLOR` を尊重）にして、
この 2 つだけ `↑`/`↓` に差し替えた。

### Q. 本家みたいに「名前は通常色＋コマンドだけ dim」の 2 色にできる？
**できない（`inquire` の構造的制約）。** 行を 2 行に分けても無理。

- `inquire` は **1 候補（option.value）まるごとに 1 つの stylesheet** しか当てない（backend.rs `print_option_value`）。
  値に改行を入れて 2 行にしても全行が同じ 1 色になるだけ。候補ごとに名前とコマンドを別アイテムにすると
  コマンド行まで選択対象になって壊れる。
- ANSI エスケープを文字列に埋め込む裏技は、`inquire` が **生文字列の幅を `UnicodeWidthChar` で数える**
  （frame_renderer.rs）ため ANSI バイトを可視幅にカウント → 折り返し誤判定・カーソルずれが起きる。
- `inquire` には「候補 1 行を自分で描画するコールバック」が無い（カスタムできるのは確定後表示の `with_formatter` だけ）。

→ 変えられるのは「行まるごと 1 色」まで（ハイライト色・全候補 dim・見出し色など RenderConfig で可）。
本家の行内 2 色を再現したいなら `promkit` 等へのバックエンド差し替えが必要。今回は切り詰め＋スクロール記号までで一旦区切り、
色分けは別タスク扱いとした。
