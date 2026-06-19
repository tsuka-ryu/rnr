use std::process::Command;

/// 組み立て済みの argv を実行し、子プロセスの exit code を返す。
///
/// `args` は `["pnpm", "run", "build", ...]` の形（空でない前提）。
/// stdio は親から継承する（.status() を使うと自動で継承され、子の出力が
/// そのまま画面に流れる）。出力はキャプチャしない。
/// シグナルで終了した場合 status.code() は None になるので、慣例として 1 を返す。
pub fn run(args: &[String]) -> anyhow::Result<i32> {
    let status = Command::new(&args[0]).args(&args[1..]).status()?;
    Ok(status.code().unwrap_or(1))
}