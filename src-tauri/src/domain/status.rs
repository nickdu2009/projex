//! Project status enum and state machine rules.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProjectStatus {
    Backlog,
    Planned,
    InProgress,
    Blocked,
    Done,
    Archived,
}

impl ProjectStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Backlog => "BACKLOG",
            Self::Planned => "PLANNED",
            Self::InProgress => "IN_PROGRESS",
            Self::Blocked => "BLOCKED",
            Self::Done => "DONE",
            Self::Archived => "ARCHIVED",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "BACKLOG" => Some(Self::Backlog),
            "PLANNED" => Some(Self::Planned),
            "IN_PROGRESS" => Some(Self::InProgress),
            "BLOCKED" => Some(Self::Blocked),
            "DONE" => Some(Self::Done),
            "ARCHIVED" => Some(Self::Archived),
            _ => None,
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Backlog,
            Self::Planned,
            Self::InProgress,
            Self::Blocked,
            Self::Done,
            Self::Archived,
        ]
    }
}

/// State machine: valid transitions and note requirements.
pub struct StatusMachine;

impl StatusMachine {
    /// Returns true if transition from `from` (None = initial) to `to` is allowed.
    pub fn can_transition(from: Option<ProjectStatus>, to: ProjectStatus) -> bool {
        use ProjectStatus::*;
        match (from, to) {
            (None, Backlog) => true, // create
            (Some(Backlog), Planned) | (Some(Backlog), Archived) => true,
            (Some(Planned), InProgress) | (Some(Planned), Archived) => true,
            (Some(InProgress), Blocked) | (Some(InProgress), Done) => true,
            (Some(Blocked), InProgress) => true,
            (Some(Done), Archived) | (Some(Done), InProgress) => true, // rework
            (Some(Archived), Backlog) => true,                       // unarchive
            _ => false,
        }
    }

    /// Returns true if this transition requires a non-empty note.
    pub fn note_required(from: Option<ProjectStatus>, to: ProjectStatus) -> bool {
        use ProjectStatus::*;
        matches!(
            (from, to),
            (Some(Archived), Backlog) |         // unarchive
            (Some(Done), InProgress) |         // rework
            (Some(Backlog), Archived) |        // abandon
            (Some(Planned), Archived)          // cancel
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_transition_create() {
        assert!(StatusMachine::can_transition(None, ProjectStatus::Backlog));
    }

    #[test]
    fn can_transition_normal_flow() {
        assert!(StatusMachine::can_transition(
            Some(ProjectStatus::Backlog),
            ProjectStatus::Planned
        ));
        assert!(StatusMachine::can_transition(
            Some(ProjectStatus::Planned),
            ProjectStatus::InProgress
        ));
        assert!(StatusMachine::can_transition(
            Some(ProjectStatus::InProgress),
            ProjectStatus::Done
        ));
    }

    #[test]
    fn invalid_transition() {
        assert!(!StatusMachine::can_transition(
            Some(ProjectStatus::Backlog),
            ProjectStatus::Done
        ));
    }

    #[test]
    fn note_required_unarchive() {
        assert!(StatusMachine::note_required(
            Some(ProjectStatus::Archived),
            ProjectStatus::Backlog
        ));
    }
}
