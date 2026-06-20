use crate::command::agent_bin;
use crate::detect::Agent;

/// ni の引数を畳んだ「意図」。argv そのものではなく操作の種類を表す。
///
/// `nr` の build_run_args が全 PM で一様だったのに対し、ni は操作ごとに argv が
/// 変わる。まず引数を「何をしたいか」に正規化し、PM 差は build_install_args が吸収する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallSpec {
    /// bare `ni` — lockfile に従って全依存をインストールする。
    All,
    /// `nci` 相当 — lockfile を凍結したまま再現インストールする。
    Frozen,
    /// `ni <pkg>` — 依存の追加。dev/global で行き先（argv）が変わる。
    Add {
        packages: Vec<String>,
        dev: bool,
        global: bool,
    },
}

/// Agent + 意図 から実行する argv を組み立てる純粋関数（ni 版 build_run_args）。
///
/// 本家 ni の `AGENTS`/`COMMANDS` 表に相当する解決テーブル。実行はせず Vec<String>
/// を返すので、--dry-run と表テストでそのまま検証できる。
/// 正解表の出どころは PLAN-NI.md の対応表（yarn は v1 基準）。
pub fn build_install_args(agent: Agent, spec: &InstallSpec) -> Vec<String> {
    let bin = agent_bin(agent).to_string();
    match spec {
        // 全部入れる: npm/pnpm は `i`、yarn/bun は `install`。
        InstallSpec::All => match agent {
            Agent::Yarn | Agent::Bun => vec![bin, "install".to_string()],
            Agent::Npm | Agent::Pnpm => vec![bin, "i".to_string()],
        },
        // frozen: npm だけ `ci`、他は install/i + --frozen-lockfile。
        InstallSpec::Frozen => match agent {
            Agent::Npm => vec![bin, "ci".to_string()],
            Agent::Pnpm => vec![bin, "i".to_string(), "--frozen-lockfile".to_string()],
            Agent::Yarn | Agent::Bun => {
                vec![bin, "install".to_string(), "--frozen-lockfile".to_string()]
            }
        },
        // 依存追加: verb（add 系）→ dev フラグ → package 名 の順で積む。
        InstallSpec::Add {
            packages,
            dev,
            global,
        } => {
            let mut args = vec![bin];
            if *global {
                // global は PM ごとに verb が違う（yarn だけ `global add`）。
                match agent {
                    Agent::Npm => {
                        args.push("i".to_string());
                        args.push("-g".to_string());
                    }
                    Agent::Pnpm | Agent::Bun => {
                        args.push("add".to_string());
                        args.push("-g".to_string());
                    }
                    Agent::Yarn => {
                        args.push("global".to_string());
                        args.push("add".to_string());
                    }
                }
            } else {
                // 通常追加: npm は `i`、他は `add`。
                match agent {
                    Agent::Npm => args.push("i".to_string()),
                    Agent::Pnpm | Agent::Yarn | Agent::Bun => args.push("add".to_string()),
                }
            }
            if *dev {
                // bun だけ小文字の `-d`、他は `-D`。
                args.push(if agent == Agent::Bun {
                    "-d".to_string()
                } else {
                    "-D".to_string()
                });
            }
            args.extend(packages.iter().cloned());
            args
        }
    }
}
