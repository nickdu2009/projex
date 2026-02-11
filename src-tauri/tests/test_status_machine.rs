//! Comprehensive status machine tests — full transition matrix

use app_lib::domain::{ProjectStatus, StatusMachine};

// ══════════════════════════════════════════════════════════
//  ProjectStatus::from_str / as_str
// ══════════════════════════════════════════════════════════

#[test]
fn from_str_all_valid_statuses() {
    assert_eq!(
        ProjectStatus::from_str("BACKLOG"),
        Some(ProjectStatus::Backlog)
    );
    assert_eq!(
        ProjectStatus::from_str("PLANNED"),
        Some(ProjectStatus::Planned)
    );
    assert_eq!(
        ProjectStatus::from_str("IN_PROGRESS"),
        Some(ProjectStatus::InProgress)
    );
    assert_eq!(
        ProjectStatus::from_str("BLOCKED"),
        Some(ProjectStatus::Blocked)
    );
    assert_eq!(ProjectStatus::from_str("DONE"), Some(ProjectStatus::Done));
    assert_eq!(
        ProjectStatus::from_str("ARCHIVED"),
        Some(ProjectStatus::Archived)
    );
}

#[test]
fn from_str_invalid_returns_none() {
    assert_eq!(ProjectStatus::from_str("FOOBAR"), None);
    assert_eq!(ProjectStatus::from_str("backlog"), None); // case-sensitive
    assert_eq!(ProjectStatus::from_str(""), None);
}

#[test]
fn as_str_roundtrips() {
    for status in ProjectStatus::all() {
        let s = status.as_str();
        assert_eq!(ProjectStatus::from_str(s), Some(*status));
    }
}

#[test]
fn all_returns_six_statuses() {
    assert_eq!(ProjectStatus::all().len(), 6);
}

// ══════════════════════════════════════════════════════════
//  can_transition — 所有合法转换
// ══════════════════════════════════════════════════════════

#[test]
fn valid_none_to_backlog() {
    assert!(StatusMachine::can_transition(None, ProjectStatus::Backlog));
}

#[test]
fn valid_backlog_to_planned() {
    assert!(StatusMachine::can_transition(
        Some(ProjectStatus::Backlog),
        ProjectStatus::Planned
    ));
}

#[test]
fn valid_backlog_to_archived() {
    assert!(StatusMachine::can_transition(
        Some(ProjectStatus::Backlog),
        ProjectStatus::Archived
    ));
}

#[test]
fn valid_planned_to_in_progress() {
    assert!(StatusMachine::can_transition(
        Some(ProjectStatus::Planned),
        ProjectStatus::InProgress
    ));
}

#[test]
fn valid_planned_to_archived() {
    assert!(StatusMachine::can_transition(
        Some(ProjectStatus::Planned),
        ProjectStatus::Archived
    ));
}

#[test]
fn valid_in_progress_to_blocked() {
    assert!(StatusMachine::can_transition(
        Some(ProjectStatus::InProgress),
        ProjectStatus::Blocked
    ));
}

#[test]
fn valid_in_progress_to_done() {
    assert!(StatusMachine::can_transition(
        Some(ProjectStatus::InProgress),
        ProjectStatus::Done
    ));
}

#[test]
fn valid_blocked_to_in_progress() {
    assert!(StatusMachine::can_transition(
        Some(ProjectStatus::Blocked),
        ProjectStatus::InProgress
    ));
}

#[test]
fn valid_done_to_archived() {
    assert!(StatusMachine::can_transition(
        Some(ProjectStatus::Done),
        ProjectStatus::Archived
    ));
}

#[test]
fn valid_done_to_in_progress_rework() {
    assert!(StatusMachine::can_transition(
        Some(ProjectStatus::Done),
        ProjectStatus::InProgress
    ));
}

#[test]
fn valid_archived_to_backlog_unarchive() {
    assert!(StatusMachine::can_transition(
        Some(ProjectStatus::Archived),
        ProjectStatus::Backlog
    ));
}

// ══════════════════════════════════════════════════════════
//  can_transition — 所有非法转换
// ══════════════════════════════════════════════════════════

#[test]
fn invalid_none_to_non_backlog() {
    assert!(!StatusMachine::can_transition(None, ProjectStatus::Planned));
    assert!(!StatusMachine::can_transition(
        None,
        ProjectStatus::InProgress
    ));
    assert!(!StatusMachine::can_transition(None, ProjectStatus::Done));
    assert!(!StatusMachine::can_transition(
        None,
        ProjectStatus::Archived
    ));
    assert!(!StatusMachine::can_transition(None, ProjectStatus::Blocked));
}

#[test]
fn invalid_backlog_to_in_progress() {
    assert!(!StatusMachine::can_transition(
        Some(ProjectStatus::Backlog),
        ProjectStatus::InProgress
    ));
}

#[test]
fn invalid_backlog_to_done() {
    assert!(!StatusMachine::can_transition(
        Some(ProjectStatus::Backlog),
        ProjectStatus::Done
    ));
}

#[test]
fn invalid_backlog_to_blocked() {
    assert!(!StatusMachine::can_transition(
        Some(ProjectStatus::Backlog),
        ProjectStatus::Blocked
    ));
}

#[test]
fn invalid_planned_to_backlog() {
    assert!(!StatusMachine::can_transition(
        Some(ProjectStatus::Planned),
        ProjectStatus::Backlog
    ));
}

#[test]
fn invalid_planned_to_done() {
    assert!(!StatusMachine::can_transition(
        Some(ProjectStatus::Planned),
        ProjectStatus::Done
    ));
}

#[test]
fn invalid_planned_to_blocked() {
    assert!(!StatusMachine::can_transition(
        Some(ProjectStatus::Planned),
        ProjectStatus::Blocked
    ));
}

#[test]
fn invalid_in_progress_to_backlog() {
    assert!(!StatusMachine::can_transition(
        Some(ProjectStatus::InProgress),
        ProjectStatus::Backlog
    ));
}

#[test]
fn invalid_in_progress_to_planned() {
    assert!(!StatusMachine::can_transition(
        Some(ProjectStatus::InProgress),
        ProjectStatus::Planned
    ));
}

#[test]
fn invalid_in_progress_to_archived() {
    assert!(!StatusMachine::can_transition(
        Some(ProjectStatus::InProgress),
        ProjectStatus::Archived
    ));
}

#[test]
fn invalid_blocked_to_backlog() {
    assert!(!StatusMachine::can_transition(
        Some(ProjectStatus::Blocked),
        ProjectStatus::Backlog
    ));
}

#[test]
fn invalid_blocked_to_planned() {
    assert!(!StatusMachine::can_transition(
        Some(ProjectStatus::Blocked),
        ProjectStatus::Planned
    ));
}

#[test]
fn invalid_blocked_to_done() {
    assert!(!StatusMachine::can_transition(
        Some(ProjectStatus::Blocked),
        ProjectStatus::Done
    ));
}

#[test]
fn invalid_blocked_to_archived() {
    assert!(!StatusMachine::can_transition(
        Some(ProjectStatus::Blocked),
        ProjectStatus::Archived
    ));
}

#[test]
fn invalid_done_to_backlog() {
    assert!(!StatusMachine::can_transition(
        Some(ProjectStatus::Done),
        ProjectStatus::Backlog
    ));
}

#[test]
fn invalid_done_to_planned() {
    assert!(!StatusMachine::can_transition(
        Some(ProjectStatus::Done),
        ProjectStatus::Planned
    ));
}

#[test]
fn invalid_done_to_blocked() {
    assert!(!StatusMachine::can_transition(
        Some(ProjectStatus::Done),
        ProjectStatus::Blocked
    ));
}

#[test]
fn invalid_archived_to_planned() {
    assert!(!StatusMachine::can_transition(
        Some(ProjectStatus::Archived),
        ProjectStatus::Planned
    ));
}

#[test]
fn invalid_archived_to_in_progress() {
    assert!(!StatusMachine::can_transition(
        Some(ProjectStatus::Archived),
        ProjectStatus::InProgress
    ));
}

#[test]
fn invalid_archived_to_done() {
    assert!(!StatusMachine::can_transition(
        Some(ProjectStatus::Archived),
        ProjectStatus::Done
    ));
}

#[test]
fn invalid_archived_to_blocked() {
    assert!(!StatusMachine::can_transition(
        Some(ProjectStatus::Archived),
        ProjectStatus::Blocked
    ));
}

#[test]
fn invalid_self_transition() {
    for status in ProjectStatus::all() {
        assert!(
            !StatusMachine::can_transition(Some(*status), *status),
            "{} -> {} should be invalid",
            status.as_str(),
            status.as_str()
        );
    }
}

// ══════════════════════════════════════════════════════════
//  note_required — 完整场景
// ══════════════════════════════════════════════════════════

#[test]
fn note_required_archived_to_backlog() {
    assert!(StatusMachine::note_required(
        Some(ProjectStatus::Archived),
        ProjectStatus::Backlog
    ));
}

#[test]
fn note_required_done_to_in_progress() {
    assert!(StatusMachine::note_required(
        Some(ProjectStatus::Done),
        ProjectStatus::InProgress
    ));
}

#[test]
fn note_required_backlog_to_archived() {
    assert!(StatusMachine::note_required(
        Some(ProjectStatus::Backlog),
        ProjectStatus::Archived
    ));
}

#[test]
fn note_required_planned_to_archived() {
    assert!(StatusMachine::note_required(
        Some(ProjectStatus::Planned),
        ProjectStatus::Archived
    ));
}

#[test]
fn note_not_required_for_normal_transitions() {
    // All valid transitions that do NOT require notes
    assert!(!StatusMachine::note_required(None, ProjectStatus::Backlog));
    assert!(!StatusMachine::note_required(
        Some(ProjectStatus::Backlog),
        ProjectStatus::Planned
    ));
    assert!(!StatusMachine::note_required(
        Some(ProjectStatus::Planned),
        ProjectStatus::InProgress
    ));
    assert!(!StatusMachine::note_required(
        Some(ProjectStatus::InProgress),
        ProjectStatus::Blocked
    ));
    assert!(!StatusMachine::note_required(
        Some(ProjectStatus::InProgress),
        ProjectStatus::Done
    ));
    assert!(!StatusMachine::note_required(
        Some(ProjectStatus::Blocked),
        ProjectStatus::InProgress
    ));
    assert!(!StatusMachine::note_required(
        Some(ProjectStatus::Done),
        ProjectStatus::Archived
    ));
}
