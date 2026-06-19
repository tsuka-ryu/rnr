use std::collections::BTreeMap;
use std::io::{self, Write};

use anyhow::Result;
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{cursor, event, execute, queue, terminal};
use event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

/// 一度に表示する候補の最大行数（本家 nr / inquire と同じ既定）。
const PAGE_SIZE: usize = 7;
/// プロンプト見出し。
const HEADER: &str = "script to run";

/// 1 つの選択候補。`haystack` は fuzzy 照合用の "key command" 連結文字列。
struct Item {
    key: String,
    command: String,
    haystack: String,
}

/// scripts 一覧から fuzzy 絞り込みで1つ選ばせる。
///
/// 本家 nr の見た目に寄せた自前 TUI:
/// - スクリプト名は通常色、コマンドは dim（グレー）の行内 2 色
/// - 長いコマンドはターミナル幅に収まるよう切り詰めて `…` を付ける
/// - 入力で fuzzy 絞り込み、↑↓ で移動、Enter で決定
///
/// `last`（直前に実行した script）が scripts に存在すれば先頭にピン留めする（本家挙動）。
/// 戻り値: 選ばれた script の key。ESC / Ctrl-C でキャンセルされたら Ok(None)。
pub fn select_script(
    scripts: &BTreeMap<String, String>,
    last: Option<&str>,
) -> Result<Option<String>> {
    // 候補リストを作る。last があれば先頭へピン留め。
    let mut items: Vec<Item> = scripts
        .iter()
        .map(|(key, command)| Item {
            key: key.clone(),
            command: command.clone(),
            haystack: format!("{key} {command}"),
        })
        .collect();
    if let Some(last) = last
        && let Some(pos) = items.iter().position(|it| it.key == last)
    {
        let item = items.remove(pos);
        items.insert(0, item);
    }

    let selected = run_picker(&items)?;
    Ok(selected.map(|idx| items[idx].key.clone()))
}

/// raw mode とカーソル表示を必ず元に戻すための RAII ガード。
struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(io::stderr(), cursor::Hide)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stderr(), cursor::Show);
        let _ = terminal::disable_raw_mode();
    }
}

/// インタラクティブな選択ループ。戻り値は items のインデックス（キャンセル時 None）。
fn run_picker(items: &[Item]) -> Result<Option<usize>> {
    let _guard = TerminalGuard::enter()?;
    let mut out = io::stderr();
    let mut matcher = Matcher::new(Config::DEFAULT);

    let mut query = String::new();
    let mut filtered: Vec<usize> = (0..items.len()).collect();
    let mut cursor = 0usize;
    let mut offset = 0usize;
    let mut prev_lines = 0u16;

    let result = loop {
        let drawn = draw(
            &mut out, items, &filtered, &query, cursor, offset, prev_lines,
        )?;
        prev_lines = drawn;

        let Event::Key(key) = event::read()? else {
            continue;
        };
        // Windows ではキー解放イベントも届くので押下/リピートだけ拾う。
        if key.kind == KeyEventKind::Release {
            continue;
        }

        match key.code {
            KeyCode::Enter => {
                break filtered.get(cursor).copied();
            }
            KeyCode::Esc => break None,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break None,
            KeyCode::Up => {
                cursor = cursor.saturating_sub(1);
            }
            KeyCode::Down if cursor + 1 < filtered.len() => {
                cursor += 1;
            }
            KeyCode::Backspace => {
                query.pop();
                filtered = filter(items, &query, &mut matcher);
                cursor = 0;
                offset = 0;
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                query.push(c);
                filtered = filter(items, &query, &mut matcher);
                cursor = 0;
                offset = 0;
            }
            _ => {}
        }

        // cursor が見える位置になるよう offset を調整。
        let page = PAGE_SIZE.min(filtered.len().max(1));
        if cursor < offset {
            offset = cursor;
        } else if cursor >= offset + page {
            offset = cursor + 1 - page;
        }
    };

    // 描画したブロックを消してから抜ける。
    if prev_lines > 0 {
        queue!(
            out,
            cursor::MoveToColumn(0),
            cursor::MoveUp(prev_lines),
            Clear(ClearType::FromCursorDown),
        )?;
        out.flush()?;
    }

    Ok(result)
}

/// query で items を絞り込み、スコア降順（同点は key 長 → key 名）で並べたインデックス列を返す。
/// query が空なら元の順序（ピン留め済み）をそのまま返す。
fn filter(items: &[Item], query: &str, matcher: &mut Matcher) -> Vec<usize> {
    if query.is_empty() {
        return (0..items.len()).collect();
    }

    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
    let mut buf: Vec<char> = Vec::new();
    let mut scored: Vec<(usize, u32)> = items
        .iter()
        .enumerate()
        .filter_map(|(i, it)| {
            buf.clear();
            pattern
                .score(Utf32Str::new(&it.haystack, &mut buf), matcher)
                .map(|score| (i, score))
        })
        .collect();

    scored.sort_by(|&(ai, asc), &(bi, bsc)| {
        bsc.cmp(&asc)
            .then_with(|| items[ai].key.len().cmp(&items[bi].key.len()))
            .then_with(|| items[ai].key.cmp(&items[bi].key))
    });

    scored.into_iter().map(|(i, _)| i).collect()
}

/// 1 フレーム描画し、描いた行数を返す（次フレームで遡って消すのに使う）。
fn draw(
    out: &mut impl Write,
    items: &[Item],
    filtered: &[usize],
    query: &str,
    cursor: usize,
    offset: usize,
    prev_lines: u16,
) -> io::Result<u16> {
    // 前フレームのブロックを消す。
    if prev_lines > 0 {
        queue!(
            out,
            cursor::MoveToColumn(0),
            cursor::MoveUp(prev_lines),
            Clear(ClearType::FromCursorDown),
        )?;
    }

    let columns = terminal_columns();

    // 見出し: "? script to run › <query>"
    queue!(
        out,
        SetForegroundColor(Color::Green),
        Print("? "),
        ResetColor,
        Print(HEADER),
        SetForegroundColor(Color::DarkGrey),
        Print(" › "),
        ResetColor,
        Print(query),
        Print("\r\n"),
    )?;
    let mut lines = 1u16;

    if filtered.is_empty() {
        queue!(
            out,
            SetForegroundColor(Color::DarkGrey),
            Print("  該当するスクリプトなし"),
            ResetColor,
            Print("\r\n"),
        )?;
        lines += 1;
    } else {
        let page = PAGE_SIZE.min(filtered.len());
        let visible = &filtered[offset..(offset + page).min(filtered.len())];
        for (row, &item_idx) in visible.iter().enumerate() {
            let absolute = offset + row;
            let selected = absolute == cursor;
            let item = &items[item_idx];

            // 行頭プレフィックス: 選択中は ❯、端ならスクロール矢印、それ以外は空白。
            if selected {
                queue!(out, SetForegroundColor(Color::Cyan), Print("❯ "), ResetColor)?;
            } else if row == 0 && offset > 0 {
                queue!(out, SetForegroundColor(Color::DarkGrey), Print("↑ "), ResetColor)?;
            } else if row + 1 == visible.len() && offset + page < filtered.len() {
                queue!(out, SetForegroundColor(Color::DarkGrey), Print("↓ "), ResetColor)?;
            } else {
                queue!(out, Print("  "))?;
            }

            // スクリプト名（選択中はシアン）。
            if selected {
                queue!(out, SetForegroundColor(Color::Cyan), Print(&item.key), ResetColor)?;
            } else {
                queue!(out, Print(&item.key))?;
            }

            // コマンド（dim グレー、折り返さないよう切り詰め）。
            // "❯ " + key + " " ぶんを引いた残り幅に収める。
            let used = 2 + item.key.chars().count() + 1;
            let command = limit_text(&item.command, columns.saturating_sub(used));
            queue!(
                out,
                Print(" "),
                SetForegroundColor(Color::DarkGrey),
                Print(command),
                ResetColor,
                Print("\r\n"),
            )?;
            lines += 1;
        }
    }

    // ヘルプ行。
    queue!(
        out,
        SetForegroundColor(Color::DarkGrey),
        Print("  ↑↓ 移動 · enter 決定 · esc 取消"),
        ResetColor,
        Print("\r\n"),
    )?;
    lines += 1;

    out.flush()?;
    Ok(lines)
}

/// 現在のターミナル幅（列数）。取得失敗時は 80 にフォールバック（本家挙動）。
fn terminal_columns() -> usize {
    terminal::size().map(|(cols, _)| cols as usize).unwrap_or(80)
}

/// `text` を表示幅 `max_width`（文字数）に収める。超える場合は切り詰めて末尾に `…`
/// を付ける（`…` も幅 1 に数える）。`max_width == 0` のときは空文字列。
fn limit_text(text: &str, max_width: usize) -> String {
    if text.chars().count() <= max_width {
        return text.to_string();
    }
    if max_width == 0 {
        return String::new();
    }
    let mut truncated: String = text.chars().take(max_width - 1).collect();
    truncated.push('…');
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_text_within_width() {
        assert_eq!(limit_text("vite build", 80), "vite build");
        assert_eq!(limit_text("abc", 3), "abc");
    }

    #[test]
    fn truncates_and_appends_ellipsis() {
        // max_width=5 なら 4 文字 + … で計 5 文字幅に収まる。
        assert_eq!(limit_text("abcdefg", 5), "abcd…");
        assert_eq!(limit_text("abcdefg", 5).chars().count(), 5);
    }

    #[test]
    fn zero_width_yields_empty() {
        assert_eq!(limit_text("anything", 0), "");
    }

    #[test]
    fn counts_by_chars_not_bytes() {
        // マルチバイトでもバイト境界で割らない。
        assert_eq!(limit_text("あいうえお", 3), "あい…");
    }

    fn items(pairs: &[(&str, &str)]) -> Vec<Item> {
        pairs
            .iter()
            .map(|(k, c)| Item {
                key: k.to_string(),
                command: c.to_string(),
                haystack: format!("{k} {c}"),
            })
            .collect()
    }

    #[test]
    fn empty_query_keeps_original_order() {
        let items = items(&[("build", "tsc"), ("dev", "vite")]);
        let mut matcher = Matcher::new(Config::DEFAULT);
        assert_eq!(filter(&items, "", &mut matcher), vec![0, 1]);
    }

    #[test]
    fn fuzzy_filters_and_ranks() {
        let items = items(&[("build", "tsc"), ("dev", "vite dev"), ("test", "vitest")]);
        let mut matcher = Matcher::new(Config::DEFAULT);
        let got = filter(&items, "dev", &mut matcher);
        // "dev"（key 完全一致）が先頭、"build" は非マッチで除外される。
        assert_eq!(got.first().copied(), Some(1));
        assert!(!got.contains(&0));
    }
}