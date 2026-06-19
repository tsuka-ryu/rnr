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