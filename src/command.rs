use std::path::Path;

use crate::detect::Agent;

/// Agent に対応する実行バイナリ名を返す。
fn agent_bin(agent: Agent) -> &'static str {
    match agent {
        Agent::Npm => "npm",
        Agent::Pnpm => "pnpm",
        Agent::Yarn => "yarn",
        Agent::Bun => "bun",
    }
}

/// Agent + script名 + 追加引数 から実行する argv を組み立てる。
///
/// `<bin> run <script> [extra...]` の形を Vec<String> で返す純粋関数。
/// 実行はしない（Phase 1 の --dry-run / テストでそのまま検証できるように分離）。
/// Phase 0 では4つの PM すべて `<bin> run <script>` で統一する
/// （yarn/bun の run 省略などの差分は後フェーズで詰める）。
pub fn build_run_args(agent: Agent, script: &str, extra: &[String]) -> Vec<String> {
    let mut args = vec![
        agent_bin(agent).to_string(),
        "run".to_string(),
        script.to_string(),
    ];
    args.extend(extra.iter().cloned());
    args
}

/// 必要なら argv の先頭に `volta run` を差し込む（純粋関数）。
///
/// 本家は volta があれば常に `volta run <cmd>` でラップするが、rnr では
/// **mise が active なら volta でラップしない**（mise に実行を委ねる）。
/// volta があり、かつ mise が非 active のときだけラップする。
/// 例: ["pnpm","run","build"] → ["volta","run","pnpm","run","build"]
pub fn wrap_volta(argv: Vec<String>, volta_available: bool, mise_active: bool) -> Vec<String> {
    if volta_available && !mise_active {
        let mut wrapped = vec!["volta".to_string(), "run".to_string()];
        wrapped.extend(argv);
        wrapped
    } else {
        argv
    }
}

/// PATH 上に実行ファイル `name` が存在するか判定する（環境を読む impure 関数）。
/// unix 前提（Windows の .exe 補完はしない）。
pub fn cmd_exists(name: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| Path::new(&dir).join(name).is_file())
}

/// mise が shell に activate されているか判定する。
/// activate 時に立つ環境変数 MISE_SHELL / __MISE_DIFF の有無で判断する。
pub fn mise_active() -> bool {
    std::env::var_os("MISE_SHELL").is_some() || std::env::var_os("__MISE_DIFF").is_some()
}