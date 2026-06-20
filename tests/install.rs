// ni 経路の表テスト: (agent, 引数) → 期待 argv。
// PLAN-NI.md の対応表をそのまま固定する（yarn は v1 基準）。

use rnr::cli::parse_ni;
use rnr::detect::Agent;
use rnr::install::{build_install_args, InstallSpec};

fn args(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

/// `rni <input>` 相当: 引数 → 意図 → argv を一気通貫で評価する。
fn ni(agent: Agent, input: &[&str]) -> Vec<String> {
    build_install_args(agent, &parse_ni(&args(input)))
}

const ALL: [Agent; 4] = [Agent::Npm, Agent::Pnpm, Agent::Yarn, Agent::Bun];

#[test]
fn bare_installs_all() {
    assert_eq!(ni(Agent::Npm, &[]), args(&["npm", "i"]));
    assert_eq!(ni(Agent::Pnpm, &[]), args(&["pnpm", "i"]));
    assert_eq!(ni(Agent::Yarn, &[]), args(&["yarn", "install"]));
    assert_eq!(ni(Agent::Bun, &[]), args(&["bun", "install"]));
}

#[test]
fn add_package() {
    assert_eq!(ni(Agent::Npm, &["axios"]), args(&["npm", "i", "axios"]));
    assert_eq!(ni(Agent::Pnpm, &["axios"]), args(&["pnpm", "add", "axios"]));
    assert_eq!(ni(Agent::Yarn, &["axios"]), args(&["yarn", "add", "axios"]));
    assert_eq!(ni(Agent::Bun, &["axios"]), args(&["bun", "add", "axios"]));
}

#[test]
fn add_dev() {
    assert_eq!(ni(Agent::Npm, &["-D", "vitest"]), args(&["npm", "i", "-D", "vitest"]));
    assert_eq!(ni(Agent::Pnpm, &["-D", "vitest"]), args(&["pnpm", "add", "-D", "vitest"]));
    assert_eq!(ni(Agent::Yarn, &["-D", "vitest"]), args(&["yarn", "add", "-D", "vitest"]));
    // bun だけ小文字 -d。
    assert_eq!(ni(Agent::Bun, &["-D", "vitest"]), args(&["bun", "add", "-d", "vitest"]));
}

#[test]
fn add_global() {
    assert_eq!(ni(Agent::Npm, &["-g", "eslint"]), args(&["npm", "i", "-g", "eslint"]));
    assert_eq!(ni(Agent::Pnpm, &["-g", "eslint"]), args(&["pnpm", "add", "-g", "eslint"]));
    // yarn v1 だけ verb が `global add`。
    assert_eq!(ni(Agent::Yarn, &["-g", "eslint"]), args(&["yarn", "global", "add", "eslint"]));
    assert_eq!(ni(Agent::Bun, &["-g", "eslint"]), args(&["bun", "add", "-g", "eslint"]));
}

#[test]
fn frozen_flag() {
    assert_eq!(ni(Agent::Npm, &["--frozen"]), args(&["npm", "ci"]));
    assert_eq!(ni(Agent::Pnpm, &["--frozen"]), args(&["pnpm", "i", "--frozen-lockfile"]));
    assert_eq!(ni(Agent::Yarn, &["--frozen"]), args(&["yarn", "install", "--frozen-lockfile"]));
    assert_eq!(ni(Agent::Bun, &["--frozen"]), args(&["bun", "install", "--frozen-lockfile"]));
}

#[test]
fn frozen_takes_priority_over_packages() {
    // --frozen があれば package 名やフラグがあっても Frozen に倒れる。
    for agent in ALL {
        assert_eq!(
            build_install_args(agent, &parse_ni(&args(&["--frozen", "-D", "axios"]))),
            build_install_args(agent, &InstallSpec::Frozen),
        );
    }
}

#[test]
fn multiple_packages_preserved_in_order() {
    assert_eq!(ni(Agent::Pnpm, &["a", "b", "c"]), args(&["pnpm", "add", "a", "b", "c"]));
}

#[test]
fn long_flags_parse() {
    assert_eq!(
        parse_ni(&args(&["--save-dev", "x"])),
        InstallSpec::Add { packages: args(&["x"]), dev: true, global: false }
    );
    assert_eq!(
        parse_ni(&args(&["--global", "x"])),
        InstallSpec::Add { packages: args(&["x"]), dev: false, global: true }
    );
}

#[test]
fn bare_g_without_package_is_all() {
    // package 名が無ければ -g 単独でも All 扱い。
    assert_eq!(parse_ni(&args(&["-g"])), InstallSpec::All);
}