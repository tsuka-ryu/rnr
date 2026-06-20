use crate::install::InstallSpec;

/// ni 経路の引数列を InstallSpec（意図）に解釈する。
///
/// 本家 ni の `parseNi` 相当。フラグを先に判定し、残りを package 名とみなす。
/// `nr` の `split_first`（先頭だけ見る）より一段複雑な、フラグ走査になる。
/// - `--frozen` / `--frozen-lockfile` → Frozen（他のフラグより優先）
/// - package 名が一つも無ければ → All（bare ni）
/// - それ以外 → Add { packages, dev, global }
///   - `-D` / `--save-dev` → dev、`-g` / `--global` → global
pub fn parse_ni(args: &[String]) -> InstallSpec {
    // frozen は単独で意味が決まるので最優先で拾う。
    if args.iter().any(|a| a == "--frozen" || a == "--frozen-lockfile") {
        return InstallSpec::Frozen;
    }

    let dev = args.iter().any(|a| a == "-D" || a == "--save-dev");
    let global = args.iter().any(|a| a == "-g" || a == "--global");

    // `-` 始まりはフラグ、それ以外を package 名とみなす。
    let packages: Vec<String> = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .cloned()
        .collect();

    // package 名なし（bare ni や `-g` 単独など）は「全部入れる」に倒す。
    if packages.is_empty() {
        return InstallSpec::All;
    }

    InstallSpec::Add {
        packages,
        dev,
        global,
    }
}