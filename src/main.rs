// rnr — a Rust reimplementation of `nr` (from @antfu-collective/ni).
// See PLAN.md for the roadmap. Implementation is intentionally left for you to fill in.
//
// Suggested module layout (create these as you go):
//   mod detect;   // lockfile -> Agent
//   mod package;  // read scripts from package.json
//   mod command;  // Agent + script -> argv (+ volta/mise handling)
//   mod runner;   // exec with inherited stdio, propagate exit code
//   mod storage;  // lastRunCommand persistence
//   mod prompt;   // interactive fuzzy script picker (Phase 2)

fn main() {
    // TODO Phase 0: parse args -> detect agent -> read scripts -> run.
    println!("rnr: not implemented yet — see PLAN.md");
}
