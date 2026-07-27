use crate::entities::label;
use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

/// Create badges with parentheses for duration
#[must_use]
pub fn create_paren_badge(text: &str) -> Span<'static> {
    Span::styled(
        format!("({text})"),
        Style::default()
            .fg(Color::Black)
            .bg(Color::LightYellow)
            .add_modifier(Modifier::BOLD),
    )
}

/// Create a label badge with custom color
#[must_use]
pub fn create_label_badge(name: &str) -> Span<'static> {
    let style = Style::default().fg(Color::Green).add_modifier(Modifier::BOLD);

    Span::styled(format!("@{}", name), style)
}

/// Create task badges optimized for terminal compatibility
#[must_use]
pub fn create_task_badges(
    is_recurring: bool,
    _has_deadline: bool,
    duration: Option<&str>,
    labels: &[label::Model],
) -> Vec<Span<'static>> {
    let mut badges = Vec::new();

    if is_recurring {
        badges.push(Span::styled("🔄", Style::default()));
    }

    if let Some(duration) = duration {
        badges.push(create_paren_badge(duration));
    }

    for label in labels {
        badges.push(create_label_badge(&label.name));
    }

    badges
}

/// Create priority badges with flag symbols
#[must_use]
pub fn create_priority_badge(priority: i32) -> Option<Span<'static>> {
    match priority {
        4 => Some(Span::styled(
            "⚑",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )), // urgent
        3 => Some(Span::styled(
            "⚑",
            Style::default().fg(Color::Rgb(255, 165, 0)).add_modifier(Modifier::BOLD),
        )), // high
        2 => Some(Span::styled(
            "⚑",
            Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
        )), // medium
        1 => Some(Span::styled("⚐", Style::default().fg(Color::White))), // low
        _ => Some(Span::styled("⚐", Style::default().fg(Color::White))), // unknown values appear low
    }
}
