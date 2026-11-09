use crate::apps_usage::UsageQuotaWindow;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum QuotaDisplay {
    #[default]
    Remaining,
    Used,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaTone {
    Normal,
    Warning,
    Danger,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TierLine<'a> {
    pub window: &'a UsageQuotaWindow,
    /// Rounded remaining percentage used to rank windows and assign their tone.
    pub remaining_percent: Option<f64>,
    /// Percentage in the selected mode. Danger lines display an exhausted label in full.
    pub shown_percent: Option<f64>,
    /// Missing utilization has no percentage or tone.
    pub tone: Option<QuotaTone>,
}

impl TierLine<'_> {
    pub fn bar_percent(&self) -> Option<f64> {
        self.shown_percent.map(|percent| percent.clamp(0.0, 100.0))
    }
}

/// Project all windows from shortest to longest, preserving input order within a duration.
pub fn tier_lines(windows: &[UsageQuotaWindow], display: QuotaDisplay) -> Vec<TierLine<'_>> {
    let mut lines: Vec<_> = windows
        .iter()
        .map(|window| {
            let remaining_percent =
                window.utilization.map(|used| (100.0 - f64::from(used)).max(0.0).round());
            let shown_percent = match display {
                QuotaDisplay::Remaining => remaining_percent,
                QuotaDisplay::Used => remaining_percent.map(|remaining| 100.0 - remaining),
            };
            let tone = remaining_percent.map(|remaining| match remaining {
                0.0 => QuotaTone::Danger,
                remaining if remaining < 20.0 => QuotaTone::Warning,
                _ => QuotaTone::Normal,
            });
            TierLine { window, remaining_percent, shown_percent, tone }
        })
        .collect();
    lines.sort_by_key(|line| window_order(&line.window.name));
    lines
}

/// Keep the shortest window on the first row. The second row fits up to two
/// other windows, chosen by lowest remaining percentage and displayed in duration order.
pub fn tier_card_rows(
    windows: &[UsageQuotaWindow],
    display: QuotaDisplay,
) -> Vec<Vec<TierLine<'_>>> {
    let lines = tier_lines(windows, display);
    if lines.len() <= 2 {
        return lines.into_iter().map(|line| vec![line]).collect();
    }

    let mut rest: Vec<_> = lines.iter().enumerate().skip(1).collect();
    rest.sort_by(|(_, a), (_, b)| {
        a.remaining_percent
            .unwrap_or(f64::INFINITY)
            .total_cmp(&b.remaining_percent.unwrap_or(f64::INFINITY))
    });
    rest.truncate(2);
    rest.sort_by_key(|(index, _)| *index);
    vec![vec![lines[0]], rest.into_iter().map(|(_, line)| *line).collect()]
}

fn window_order(name: &str) -> u8 {
    match name {
        "five_hour" | "agy_gemini_five_hour" | "agy_third_party_five_hour" => 0,
        "gemini_pro" | "gemini_flash" | "gemini_flash_lite" => 1,
        "seven_day"
        | "seven_day_fable"
        | "seven_day_opus"
        | "seven_day_sonnet"
        | "weekly_limit"
        | "agy_gemini_weekly"
        | "agy_third_party_weekly" => 2,
        "30_day" | "monthly" | "credits" | "premium" => 3,
        _ => 4,
    }
}
