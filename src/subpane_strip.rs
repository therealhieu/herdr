//! Subpane strip: the member list a stacked pane slot draws on its top border.
//!
//! `strip_layout` is pure: segment geometry depends only on the member labels,
//! the active index, and the slot width, so the server renderer and the client
//! hit-testing compute identical ranges.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::api::schema::AgentStatus;
use crate::app::state::Palette;
use crate::config::StatusIndicatorStyle;
use crate::terminal::TerminalState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StripMode {
    /// Every member, the close button, and the add button fit.
    Full,
    /// Only the active member plus a `[i/n]` badge fit.
    Badge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StripSegmentKind {
    /// ` {status} {label} ` for member `i`.
    Member(usize),
    /// `× ` closing member `i` (always the active member).
    Close(usize),
    /// ` + ` adding a member.
    Add,
    /// `[i/n {status}]` showing the next member.
    Badge,
}

/// One clickable range of the strip. `x` is the column offset from the slot's
/// left edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StripSegment {
    pub x: u16,
    pub width: u16,
    pub kind: StripSegmentKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StripLayout {
    pub mode: StripMode,
    pub segments: Vec<StripSegment>,
}

/// Longest member label, in columns.
pub(crate) const MAX_LABEL_WIDTH: usize = 20;
pub(crate) const FALLBACK_LABEL: &str = "shell";
/// ` {status} ` before the label and ` ` after it.
const MEMBER_CHROME: u16 = 4;
/// `× ` after the active member's label.
const CLOSE_WIDTH: u16 = 2;
/// ` + `.
const ADD_WIDTH: u16 = 3;
/// One border cell between segments.
const SEPARATOR: u16 = 1;

/// Lay out the strip for `labels` on a slot `width` columns wide. The strip
/// starts one column right of the top-left corner and never reaches the
/// top-right corner. Returns no segments for fewer than two members, an
/// out-of-range `active`, or a slot too narrow for even the badge.
pub(crate) fn strip_layout(labels: &[&str], active: usize, width: u16) -> StripLayout {
    let empty = |mode| StripLayout {
        mode,
        segments: Vec::new(),
    };
    if labels.len() < 2 || active >= labels.len() {
        return empty(StripMode::Full);
    }
    let inner = width.saturating_sub(2);
    let label_widths: Vec<u16> = labels.iter().map(|label| label_width(label)).collect();

    let full_width = label_widths
        .iter()
        .map(|width| u32::from(width + MEMBER_CHROME + SEPARATOR))
        .sum::<u32>()
        + u32::from(CLOSE_WIDTH + ADD_WIDTH);
    if full_width <= u32::from(inner) {
        let mut segments = Vec::with_capacity(labels.len() + 2);
        let mut x = 1;
        for (idx, label_width) in label_widths.iter().enumerate() {
            let member_width = label_width + MEMBER_CHROME;
            segments.push(StripSegment {
                x,
                width: member_width,
                kind: StripSegmentKind::Member(idx),
            });
            x += member_width;
            if idx == active {
                segments.push(StripSegment {
                    x,
                    width: CLOSE_WIDTH,
                    kind: StripSegmentKind::Close(idx),
                });
                x += CLOSE_WIDTH;
            }
            x += SEPARATOR;
        }
        segments.push(StripSegment {
            x,
            width: ADD_WIDTH,
            kind: StripSegmentKind::Add,
        });
        return StripLayout {
            mode: StripMode::Full,
            segments,
        };
    }

    // `[i/n s]`, with the status column always reserved so the badge does
    // not move when hidden statuses change.
    let badge_width = u16::try_from(badge_text(active, labels.len()).len() + 4).unwrap_or(u16::MAX);
    // Badge ends one border cell before the top-right corner.
    let badge_x = width.saturating_sub(2 + badge_width);
    let fixed = 1 + MEMBER_CHROME + CLOSE_WIDTH + SEPARATOR;
    let room = badge_x.saturating_sub(fixed);
    if badge_x < fixed || room == 0 {
        return empty(StripMode::Badge);
    }
    let member_width = label_widths[active].min(room) + MEMBER_CHROME;
    StripLayout {
        mode: StripMode::Badge,
        segments: vec![
            StripSegment {
                x: 1,
                width: member_width,
                kind: StripSegmentKind::Member(active),
            },
            StripSegment {
                x: 1 + member_width,
                width: CLOSE_WIDTH,
                kind: StripSegmentKind::Close(active),
            },
            StripSegment {
                x: badge_x,
                width: badge_width,
                kind: StripSegmentKind::Badge,
            },
        ],
    }
}

/// `{i}/{n}` with a 1-based `i`, the badge text without brackets or status.
pub(crate) fn badge_text(active: usize, count: usize) -> String {
    format!("{}/{}", active + 1, count)
}

/// Columns a member segment's label may use.
pub(crate) fn member_label_width(segment: &StripSegment) -> usize {
    usize::from(segment.width.saturating_sub(MEMBER_CHROME))
}

fn label_width(label: &str) -> u16 {
    let width = UnicodeWidthStr::width(label).min(MAX_LABEL_WIDTH);
    width as u16
}

/// Truncate `label` to `max_width` columns, ending with `…` when cut.
pub(crate) fn fit_label(label: &str, max_width: usize) -> String {
    if UnicodeWidthStr::width(label) <= max_width {
        return label.to_string();
    }
    if max_width == 0 {
        return String::new();
    }
    let mut output = String::new();
    let mut used = 0;
    for ch in label.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + ch_width > max_width - 1 {
            break;
        }
        output.push(ch);
        used += ch_width;
    }
    output.push('…');
    output
}

/// A member's strip label: manual label, then the detected agent's display
/// name, then the terminal title, then `shell`, truncated to
/// `MAX_LABEL_WIDTH` columns.
pub(crate) fn subpane_label(terminal: &TerminalState) -> String {
    let label = terminal
        .manual_label
        .clone()
        .or_else(|| {
            let agent = terminal.effective_agent_label()?;
            Some(
                terminal
                    .effective_display_agent()
                    .unwrap_or_else(|| agent.to_string()),
            )
        })
        .or_else(|| terminal.terminal_title_stripped())
        .unwrap_or_else(|| FALLBACK_LABEL.to_string());
    fit_label(label.trim(), MAX_LABEL_WIDTH)
}

/// What the strip shows for one member. `status` is `None` when no agent is
/// detected, which leaves the status column blank.
pub(crate) struct StripMember {
    pub label: String,
    pub status: Option<AgentStatus>,
}

pub(crate) struct StripStyle<'a> {
    pub focused: bool,
    pub palette: &'a Palette,
    pub indicators: StatusIndicatorStyle,
}

/// Draw the strip over the top border of the slot at `rect`, clipped to the
/// buffer and to the columns between the slot's corners.
pub(crate) fn render_strip(
    buf: &mut Buffer,
    rect: Rect,
    members: &[StripMember],
    active: usize,
    style: StripStyle<'_>,
) {
    let area = buf.area;
    if rect.y < area.y || rect.y >= area.y.saturating_add(area.height) {
        return;
    }
    let labels: Vec<&str> = members.iter().map(|member| member.label.as_str()).collect();
    let layout = strip_layout(&labels, active, rect.width);
    let end_x = rect
        .x
        .saturating_add(rect.width.saturating_sub(1))
        .min(area.x.saturating_add(area.width));
    let mut put = |x: u16, text: &str, text_style: Style| {
        let x = rect.x.saturating_add(x);
        if x >= area.x && x < end_x {
            buf.set_stringn(x, rect.y, text, usize::from(end_x - x), text_style);
        }
    };
    let palette = style.palette;
    let label_style = |idx: usize| match (idx == active, style.focused) {
        (true, true) => Style::default()
            .fg(palette.accent)
            .add_modifier(Modifier::BOLD),
        (true, false) => Style::default()
            .fg(palette.text)
            .add_modifier(Modifier::BOLD),
        (false, _) => Style::default().fg(palette.overlay0),
    };
    let glyph = |status: AgentStatus| {
        (
            crate::client::status_icon(status, style.indicators),
            Style::default().fg(crate::client::status_color(status, palette)),
        )
    };

    for segment in &layout.segments {
        match segment.kind {
            StripSegmentKind::Member(idx) => {
                let Some(member) = members.get(idx) else {
                    continue;
                };
                let text_style = label_style(idx);
                let label = fit_label(&member.label, member_label_width(segment));
                put(segment.x, &format!("   {label} "), text_style);
                if let Some(status) = member.status {
                    let (icon, icon_style) = glyph(status);
                    put(segment.x + 1, icon, icon_style);
                }
            }
            StripSegmentKind::Close(idx) => put(segment.x, "× ", label_style(idx)),
            StripSegmentKind::Add => put(segment.x, " + ", Style::default().fg(palette.overlay0)),
            StripSegmentKind::Badge => {
                let text = badge_text(active, members.len());
                let hidden = (layout.mode == StripMode::Badge)
                    .then(|| most_urgent_hidden_status(members, active))
                    .flatten();
                let badge_style = Style::default().fg(palette.overlay0);
                let right = segment.x + segment.width;
                match hidden {
                    Some(status) => {
                        let (icon, icon_style) = glyph(status);
                        put(segment.x, &format!("[{text} "), badge_style);
                        put(right - 2, icon, icon_style);
                        put(right - 1, "]", badge_style);
                    }
                    None => {
                        let text = format!("[{text}]");
                        let width = u16::try_from(text.len()).unwrap_or(segment.width);
                        put(right.saturating_sub(width), &text, badge_style);
                    }
                }
            }
        }
    }
}

/// The highest-priority status among hidden members, ignoring idle members
/// and members without an agent.
fn most_urgent_hidden_status(members: &[StripMember], active: usize) -> Option<AgentStatus> {
    let idle = crate::client::status_priority(AgentStatus::Idle);
    members
        .iter()
        .enumerate()
        .filter(|(idx, _)| *idx != active)
        .filter_map(|(_, member)| member.status)
        .filter(|status| crate::client::status_priority(*status) > idle)
        .max_by_key(|status| crate::client::status_priority(*status))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detect::{Agent, AgentState};
    use StripSegmentKind::{Add, Badge, Close, Member};

    fn seg(x: u16, width: u16, kind: StripSegmentKind) -> StripSegment {
        StripSegment { x, width, kind }
    }

    const LABELS: [&str; 3] = ["claude", "codex", "zsh"];

    #[test]
    fn subpane_strip_full_mode_places_members_close_and_add() {
        // ┌ ● claude × ─ ● codex ─ ● zsh ─ + ┐ : 34 columns between corners.
        let layout = strip_layout(&LABELS, 0, 36);

        assert_eq!(layout.mode, StripMode::Full);
        assert_eq!(
            layout.segments,
            vec![
                seg(1, 10, Member(0)),
                seg(11, 2, Close(0)),
                seg(14, 9, Member(1)),
                seg(24, 7, Member(2)),
                seg(32, 3, Add),
            ]
        );

        let layout = strip_layout(&LABELS, 1, 36);
        assert_eq!(
            layout.segments,
            vec![
                seg(1, 10, Member(0)),
                seg(12, 9, Member(1)),
                seg(21, 2, Close(1)),
                seg(24, 7, Member(2)),
                seg(32, 3, Add),
            ]
        );
    }

    #[test]
    fn subpane_strip_one_column_short_switches_to_badge() {
        let layout = strip_layout(&LABELS, 0, 35);

        assert_eq!(layout.mode, StripMode::Badge);
        assert_eq!(
            layout.segments,
            vec![
                seg(1, 10, Member(0)),
                seg(11, 2, Close(0)),
                seg(26, 7, Badge),
            ]
        );
    }

    #[test]
    fn subpane_strip_badge_truncates_active_label() {
        let layout = strip_layout(&["claude-code", "zsh"], 0, 20);

        assert_eq!(layout.mode, StripMode::Badge);
        assert_eq!(
            layout.segments,
            vec![seg(1, 7, Member(0)), seg(8, 2, Close(0)), seg(11, 7, Badge)]
        );
        assert_eq!(fit_label("claude-code", 7 - 4), "cl…");
    }

    #[test]
    fn subpane_strip_below_minimum_width_has_no_segments() {
        assert!(strip_layout(&LABELS, 0, 17).segments.is_empty());
        assert_eq!(strip_layout(&LABELS, 0, 18).segments.len(), 3);
        assert!(strip_layout(&["solo"], 0, 80).segments.is_empty());
        assert!(strip_layout(&LABELS, 3, 80).segments.is_empty());
    }

    #[test]
    fn subpane_strip_label_follows_manual_agent_title_shell_order() {
        let mut terminal = TerminalState::new(
            crate::terminal::TerminalId::alloc(),
            std::path::PathBuf::from("/tmp"),
        );
        assert_eq!(subpane_label(&terminal), "shell");

        terminal.terminal_title = Some("vim notes.md".into());
        assert_eq!(subpane_label(&terminal), "vim notes.md");

        terminal.set_detected_state(Some(Agent::Claude), AgentState::Idle);
        assert_eq!(subpane_label(&terminal), "claude");

        terminal.set_manual_label("a-very-long-reviewer-label".into());
        assert_eq!(subpane_label(&terminal), "a-very-long-reviewe…");
    }
}
