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