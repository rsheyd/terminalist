//! Human-readable task priorities and Todoist API conversion.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Urgent,
}

impl TaskPriority {
    #[must_use]
    pub fn from_todoist(value: i32) -> Self {
        match value {
            4 => Self::Urgent,
            3 => Self::High,
            2 => Self::Medium,
            _ => Self::Low,
        }
    }

    #[must_use]
    pub const fn todoist_value(self) -> i32 {
        match self {
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
            Self::Urgent => 4,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Urgent => "urgent",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TaskPriority;

    #[test]
    fn maps_named_priorities_to_todoist_values() {
        let cases = [
            (TaskPriority::Low, 1),
            (TaskPriority::Medium, 2),
            (TaskPriority::High, 3),
            (TaskPriority::Urgent, 4),
        ];

        for (priority, todoist_value) in cases {
            assert_eq!(priority.todoist_value(), todoist_value);
            assert_eq!(TaskPriority::from_todoist(todoist_value), priority);
        }
    }
}
