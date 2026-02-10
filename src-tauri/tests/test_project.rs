//! Project CRUD + status machine integration tests

use app_lib::app::{
    partner_create, person_create,
    project_create, project_get, project_update, project_list, project_change_status,
    PartnerCreateReq, PersonCreateReq,
    ProjectCreateReq, ProjectUpdateReq, ProjectListReq, ProjectChangeStatusReq,
};
use app_lib::infra::db::init_test_db;

// ──────────────────────── Helper ────────────────────────

struct TestSeedIds {
    person_id: String,
    partner_id: String,
}

fn seed(pool: &app_lib::infra::DbPool) -> TestSeedIds {
    let person = person_create(pool, PersonCreateReq {
        display_name: "Owner".to_string(),
        email: Some("owner@test.com".to_string()),
        role: Some("PM".to_string()),
        note: None,
    }).unwrap();
    let partner = partner_create(pool, PartnerCreateReq {
        name: format!("Partner-{}", uuid::Uuid::new_v4()),
        note: None,
    }).unwrap();
    TestSeedIds {
        person_id: person.id,
        partner_id: partner.id,
    }
}

fn make_project_req(ids: &TestSeedIds, name: &str) -> ProjectCreateReq {
    ProjectCreateReq {
        name: name.to_string(),
        description: Some("desc".to_string()),
        priority: Some(3),
        country_code: "CN".to_string(),
        partner_id: ids.partner_id.clone(),
        owner_person_id: ids.person_id.clone(),
        start_date: Some("2026-01-01".to_string()),
        due_date: Some("2026-12-31".to_string()),
        tags: Some(vec!["tag1".to_string(), "tag2".to_string()]),
        created_by_person_id: Some(ids.person_id.clone()),
    }
}

// ══════════════════════════════════════════════════════════
//  project_create
// ══════════════════════════════════════════════════════════

#[test]
fn create_project_returns_full_detail() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let proj = project_create(&pool, make_project_req(&ids, "Project Alpha")).unwrap();

    assert_eq!(proj.name, "Project Alpha");
    assert_eq!(proj.current_status, "BACKLOG");
    assert_eq!(proj.priority, 3);
    assert_eq!(proj.country_code, "CN");
    assert_eq!(proj.partner_id, ids.partner_id);
    assert_eq!(proj.owner_person_id, ids.person_id);
    assert_eq!(proj.start_date, Some("2026-01-01".to_string()));
    assert_eq!(proj.due_date, Some("2026-12-31".to_string()));
    assert!(proj.archived_at.is_none());
    assert_eq!(proj.tags.len(), 2);
    assert!(proj.tags.contains(&"tag1".to_string()));
    assert!(proj.tags.contains(&"tag2".to_string()));
    assert_eq!(proj.owner_name, "Owner");
}

#[test]
fn create_project_auto_creates_owner_assignment() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let proj = project_create(&pool, make_project_req(&ids, "AssignTest")).unwrap();

    assert_eq!(proj.assignments.len(), 1);
    assert_eq!(proj.assignments[0].person_id, ids.person_id);
    assert_eq!(proj.assignments[0].role, "owner");
    assert!(proj.assignments[0].end_at.is_none());
}

#[test]
fn create_project_auto_creates_status_history() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let proj = project_create(&pool, make_project_req(&ids, "HistTest")).unwrap();

    assert_eq!(proj.status_history.len(), 1);
    assert_eq!(proj.status_history[0].to_status, "BACKLOG");
    assert!(proj.status_history[0].from_status.is_none());
}

#[test]
fn create_project_empty_name_fails() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let mut req = make_project_req(&ids, "");
    req.name = "   ".to_string();
    let err = project_create(&pool, req);
    assert_eq!(err.unwrap_err().code(), "VALIDATION_ERROR");
}

#[test]
fn create_project_empty_country_code_fails() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let mut req = make_project_req(&ids, "NoCountry");
    req.country_code = "  ".to_string();
    let err = project_create(&pool, req);
    assert_eq!(err.unwrap_err().code(), "VALIDATION_ERROR");
}

#[test]
fn create_project_priority_clamped() {
    let pool = init_test_db();
    let ids = seed(&pool);

    let mut req = make_project_req(&ids, "HighPri");
    req.priority = Some(99);
    let proj = project_create(&pool, req).unwrap();
    assert_eq!(proj.priority, 5);

    let mut req = make_project_req(&ids, "LowPri");
    req.priority = Some(-1);
    let proj = project_create(&pool, req).unwrap();
    assert_eq!(proj.priority, 1);
}

#[test]
fn create_project_country_code_uppercased() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let mut req = make_project_req(&ids, "Upper");
    req.country_code = "jp".to_string();
    let proj = project_create(&pool, req).unwrap();
    assert_eq!(proj.country_code, "JP");
}

#[test]
fn create_project_empty_tags_ignored() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let mut req = make_project_req(&ids, "EmptyTags");
    req.tags = Some(vec!["  ".to_string(), "valid".to_string(), "".to_string()]);
    let proj = project_create(&pool, req).unwrap();
    assert_eq!(proj.tags, vec!["valid"]);
}

// ══════════════════════════════════════════════════════════
//  project_get
// ══════════════════════════════════════════════════════════

#[test]
fn get_project_not_found() {
    let pool = init_test_db();
    let err = project_get(&pool, "nonexistent");
    assert_eq!(err.unwrap_err().code(), "NOT_FOUND");
}

// ══════════════════════════════════════════════════════════
//  project_update
// ══════════════════════════════════════════════════════════

#[test]
fn update_project_partial_fields() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let proj = project_create(&pool, make_project_req(&ids, "Original")).unwrap();

    let updated = project_update(&pool, ProjectUpdateReq {
        id: proj.id.clone(),
        name: Some("Renamed".to_string()),
        description: None,
        priority: Some(5),
        country_code: None,
        owner_person_id: None,
        start_date: None,
        due_date: None,
        tags: None,
        partner_id: None,
    }).unwrap();

    assert_eq!(updated.name, "Renamed");
    assert_eq!(updated.priority, 5);
    assert_eq!(updated.description, "desc"); // unchanged
    assert_eq!(updated.country_code, "CN");  // unchanged
}

#[test]
fn update_project_partner_id_immutable() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let proj = project_create(&pool, make_project_req(&ids, "Immutable")).unwrap();

    let err = project_update(&pool, ProjectUpdateReq {
        id: proj.id.clone(),
        name: None,
        description: None,
        priority: None,
        country_code: None,
        owner_person_id: None,
        start_date: None,
        due_date: None,
        tags: None,
        partner_id: Some("new-partner-id".to_string()),
    });
    assert_eq!(err.unwrap_err().code(), "PARTNER_IMMUTABLE");
}

#[test]
fn update_project_owner_change_demotes_old_owner() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let proj = project_create(&pool, make_project_req(&ids, "OwnerChange")).unwrap();

    // Create a new person to be the new owner
    let new_owner = person_create(&pool, PersonCreateReq {
        display_name: "New Owner".to_string(),
        email: None, role: None, note: None,
    }).unwrap();

    let updated = project_update(&pool, ProjectUpdateReq {
        id: proj.id.clone(),
        name: None,
        description: None,
        priority: None,
        country_code: None,
        owner_person_id: Some(new_owner.id.clone()),
        start_date: None,
        due_date: None,
        tags: None,
        partner_id: None,
    }).unwrap();

    assert_eq!(updated.owner_person_id, new_owner.id);
    // Check assignments: old owner should be 'member', new owner should be 'owner'
    let old_owner_asgn = updated.assignments.iter()
        .find(|a| a.person_id == ids.person_id && a.end_at.is_none())
        .unwrap();
    assert_eq!(old_owner_asgn.role, "member");

    let new_owner_asgn = updated.assignments.iter()
        .find(|a| a.person_id == new_owner.id && a.end_at.is_none())
        .unwrap();
    assert_eq!(new_owner_asgn.role, "owner");
}

#[test]
fn update_project_tags_replaced() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let proj = project_create(&pool, make_project_req(&ids, "TagReplace")).unwrap();
    assert_eq!(proj.tags.len(), 2);

    let updated = project_update(&pool, ProjectUpdateReq {
        id: proj.id.clone(),
        name: None,
        description: None,
        priority: None,
        country_code: None,
        owner_person_id: None,
        start_date: None,
        due_date: None,
        tags: Some(vec!["new-tag".to_string()]),
        partner_id: None,
    }).unwrap();

    assert_eq!(updated.tags, vec!["new-tag"]);
}

// ══════════════════════════════════════════════════════════
//  project_list
// ══════════════════════════════════════════════════════════

#[test]
fn list_projects_returns_items() {
    let pool = init_test_db();
    let ids = seed(&pool);
    project_create(&pool, make_project_req(&ids, "P1")).unwrap();
    project_create(&pool, make_project_req(&ids, "P2")).unwrap();

    let page = project_list(&pool, ProjectListReq::default()).unwrap();
    assert_eq!(page.items.len(), 2);
    assert_eq!(page.total, 2);
}

#[test]
fn list_projects_excludes_archived_by_default() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let p1 = project_create(&pool, make_project_req(&ids, "Active")).unwrap();
    let p2 = project_create(&pool, make_project_req(&ids, "ToArchive")).unwrap();

    // Move p2 through status flow to ARCHIVED
    project_change_status(&pool, ProjectChangeStatusReq {
        project_id: p2.id.clone(),
        to_status: "PLANNED".to_string(),
        note: None,
        changed_by_person_id: None,
        if_match_updated_at: None,
    }).unwrap();
    project_change_status(&pool, ProjectChangeStatusReq {
        project_id: p2.id.clone(),
        to_status: "IN_PROGRESS".to_string(),
        note: None,
        changed_by_person_id: None,
        if_match_updated_at: None,
    }).unwrap();
    project_change_status(&pool, ProjectChangeStatusReq {
        project_id: p2.id.clone(),
        to_status: "DONE".to_string(),
        note: None,
        changed_by_person_id: None,
        if_match_updated_at: None,
    }).unwrap();
    project_change_status(&pool, ProjectChangeStatusReq {
        project_id: p2.id.clone(),
        to_status: "ARCHIVED".to_string(),
        note: None,
        changed_by_person_id: None,
        if_match_updated_at: None,
    }).unwrap();

    // Default (only_unarchived = true)
    let page = project_list(&pool, ProjectListReq::default()).unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].id, p1.id);

    // Explicit include archived
    let page_all = project_list(&pool, ProjectListReq {
        only_unarchived: Some(false),
        ..Default::default()
    }).unwrap();
    assert_eq!(page_all.items.len(), 2);
}

#[test]
fn list_projects_includes_tags() {
    let pool = init_test_db();
    let ids = seed(&pool);
    project_create(&pool, make_project_req(&ids, "Tagged")).unwrap();

    let page = project_list(&pool, ProjectListReq::default()).unwrap();
    assert!(!page.items[0].tags.is_empty());
}

// ══════════════════════════════════════════════════════════
//  project_change_status (状态机)
// ══════════════════════════════════════════════════════════

#[test]
fn change_status_normal_flow() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let proj = project_create(&pool, make_project_req(&ids, "Flow")).unwrap();
    assert_eq!(proj.current_status, "BACKLOG");

    // BACKLOG → PLANNED
    let p = project_change_status(&pool, ProjectChangeStatusReq {
        project_id: proj.id.clone(),
        to_status: "PLANNED".to_string(),
        note: None,
        changed_by_person_id: None,
        if_match_updated_at: None,
    }).unwrap();
    assert_eq!(p.current_status, "PLANNED");

    // PLANNED → IN_PROGRESS
    let p = project_change_status(&pool, ProjectChangeStatusReq {
        project_id: proj.id.clone(),
        to_status: "IN_PROGRESS".to_string(),
        note: None,
        changed_by_person_id: None,
        if_match_updated_at: None,
    }).unwrap();
    assert_eq!(p.current_status, "IN_PROGRESS");

    // IN_PROGRESS → DONE
    let p = project_change_status(&pool, ProjectChangeStatusReq {
        project_id: proj.id.clone(),
        to_status: "DONE".to_string(),
        note: None,
        changed_by_person_id: None,
        if_match_updated_at: None,
    }).unwrap();
    assert_eq!(p.current_status, "DONE");

    // DONE → ARCHIVED
    let p = project_change_status(&pool, ProjectChangeStatusReq {
        project_id: proj.id.clone(),
        to_status: "ARCHIVED".to_string(),
        note: None,
        changed_by_person_id: None,
        if_match_updated_at: None,
    }).unwrap();
    assert_eq!(p.current_status, "ARCHIVED");
    assert!(p.archived_at.is_some());
}

#[test]
fn change_status_invalid_transition_rejected() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let proj = project_create(&pool, make_project_req(&ids, "Invalid")).unwrap();

    // BACKLOG → DONE (not allowed)
    let err = project_change_status(&pool, ProjectChangeStatusReq {
        project_id: proj.id.clone(),
        to_status: "DONE".to_string(),
        note: None,
        changed_by_person_id: None,
        if_match_updated_at: None,
    });
    assert_eq!(err.unwrap_err().code(), "INVALID_STATUS_TRANSITION");
}

#[test]
fn change_status_unknown_status_rejected() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let proj = project_create(&pool, make_project_req(&ids, "Unknown")).unwrap();

    let err = project_change_status(&pool, ProjectChangeStatusReq {
        project_id: proj.id.clone(),
        to_status: "FOOBAR".to_string(),
        note: None,
        changed_by_person_id: None,
        if_match_updated_at: None,
    });
    assert_eq!(err.unwrap_err().code(), "INVALID_STATUS_TRANSITION");
}

#[test]
fn change_status_note_required_for_abandon() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let proj = project_create(&pool, make_project_req(&ids, "Abandon")).unwrap();

    // BACKLOG → ARCHIVED requires note
    let err = project_change_status(&pool, ProjectChangeStatusReq {
        project_id: proj.id.clone(),
        to_status: "ARCHIVED".to_string(),
        note: None,
        changed_by_person_id: None,
        if_match_updated_at: None,
    });
    assert_eq!(err.unwrap_err().code(), "NOTE_REQUIRED");

    // With note: should succeed
    let p = project_change_status(&pool, ProjectChangeStatusReq {
        project_id: proj.id.clone(),
        to_status: "ARCHIVED".to_string(),
        note: Some("Abandoned due to budget cuts".to_string()),
        changed_by_person_id: None,
        if_match_updated_at: None,
    }).unwrap();
    assert_eq!(p.current_status, "ARCHIVED");
}

#[test]
fn change_status_note_required_for_unarchive() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let proj = project_create(&pool, make_project_req(&ids, "Unarchive")).unwrap();

    // Archive first
    project_change_status(&pool, ProjectChangeStatusReq {
        project_id: proj.id.clone(),
        to_status: "ARCHIVED".to_string(),
        note: Some("archive reason".to_string()),
        changed_by_person_id: None,
        if_match_updated_at: None,
    }).unwrap();

    // ARCHIVED → BACKLOG without note fails
    let err = project_change_status(&pool, ProjectChangeStatusReq {
        project_id: proj.id.clone(),
        to_status: "BACKLOG".to_string(),
        note: None,
        changed_by_person_id: None,
        if_match_updated_at: None,
    });
    assert_eq!(err.unwrap_err().code(), "NOTE_REQUIRED");

    // With note succeeds
    let p = project_change_status(&pool, ProjectChangeStatusReq {
        project_id: proj.id.clone(),
        to_status: "BACKLOG".to_string(),
        note: Some("Reviving project".to_string()),
        changed_by_person_id: None,
        if_match_updated_at: None,
    }).unwrap();
    assert_eq!(p.current_status, "BACKLOG");
}

#[test]
fn change_status_note_required_for_rework() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let proj = project_create(&pool, make_project_req(&ids, "Rework")).unwrap();

    // Move to DONE
    project_change_status(&pool, ProjectChangeStatusReq {
        project_id: proj.id.clone(), to_status: "PLANNED".to_string(),
        note: None, changed_by_person_id: None, if_match_updated_at: None,
    }).unwrap();
    project_change_status(&pool, ProjectChangeStatusReq {
        project_id: proj.id.clone(), to_status: "IN_PROGRESS".to_string(),
        note: None, changed_by_person_id: None, if_match_updated_at: None,
    }).unwrap();
    project_change_status(&pool, ProjectChangeStatusReq {
        project_id: proj.id.clone(), to_status: "DONE".to_string(),
        note: None, changed_by_person_id: None, if_match_updated_at: None,
    }).unwrap();

    // DONE → IN_PROGRESS without note fails
    let err = project_change_status(&pool, ProjectChangeStatusReq {
        project_id: proj.id.clone(), to_status: "IN_PROGRESS".to_string(),
        note: None, changed_by_person_id: None, if_match_updated_at: None,
    });
    assert_eq!(err.unwrap_err().code(), "NOTE_REQUIRED");

    // With note succeeds
    let p = project_change_status(&pool, ProjectChangeStatusReq {
        project_id: proj.id.clone(), to_status: "IN_PROGRESS".to_string(),
        note: Some("Found bugs".to_string()),
        changed_by_person_id: None, if_match_updated_at: None,
    }).unwrap();
    assert_eq!(p.current_status, "IN_PROGRESS");
}

#[test]
fn change_status_blocked_and_unblocked() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let proj = project_create(&pool, make_project_req(&ids, "Blocked")).unwrap();

    // Move to IN_PROGRESS
    project_change_status(&pool, ProjectChangeStatusReq {
        project_id: proj.id.clone(), to_status: "PLANNED".to_string(),
        note: None, changed_by_person_id: None, if_match_updated_at: None,
    }).unwrap();
    project_change_status(&pool, ProjectChangeStatusReq {
        project_id: proj.id.clone(), to_status: "IN_PROGRESS".to_string(),
        note: None, changed_by_person_id: None, if_match_updated_at: None,
    }).unwrap();

    // IN_PROGRESS → BLOCKED
    let p = project_change_status(&pool, ProjectChangeStatusReq {
        project_id: proj.id.clone(), to_status: "BLOCKED".to_string(),
        note: None, changed_by_person_id: None, if_match_updated_at: None,
    }).unwrap();
    assert_eq!(p.current_status, "BLOCKED");

    // BLOCKED → IN_PROGRESS
    let p = project_change_status(&pool, ProjectChangeStatusReq {
        project_id: proj.id.clone(), to_status: "IN_PROGRESS".to_string(),
        note: None, changed_by_person_id: None, if_match_updated_at: None,
    }).unwrap();
    assert_eq!(p.current_status, "IN_PROGRESS");
}

#[test]
fn change_status_optimistic_lock_conflict() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let proj = project_create(&pool, make_project_req(&ids, "OptLock")).unwrap();

    let err = project_change_status(&pool, ProjectChangeStatusReq {
        project_id: proj.id.clone(),
        to_status: "PLANNED".to_string(),
        note: None,
        changed_by_person_id: None,
        if_match_updated_at: Some("1970-01-01T00:00:00Z".to_string()), // stale
    });
    assert_eq!(err.unwrap_err().code(), "CONFLICT");
}

#[test]
fn change_status_records_history() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let proj = project_create(&pool, make_project_req(&ids, "History")).unwrap();

    project_change_status(&pool, ProjectChangeStatusReq {
        project_id: proj.id.clone(),
        to_status: "PLANNED".to_string(),
        note: Some("Planning started".to_string()),
        changed_by_person_id: Some(ids.person_id.clone()),
        if_match_updated_at: None,
    }).unwrap();

    let detail = project_get(&pool, &proj.id).unwrap();
    // Should have 2 history entries: initial BACKLOG + transition to PLANNED
    assert_eq!(detail.status_history.len(), 2);

    let latest = &detail.status_history[0]; // ordered by changed_at DESC
    assert_eq!(latest.from_status, Some("BACKLOG".to_string()));
    assert_eq!(latest.to_status, "PLANNED");
    assert_eq!(latest.note, "Planning started");
    assert_eq!(latest.changed_by_person_id, Some(ids.person_id.clone()));
}

#[test]
fn change_status_not_found_project() {
    let pool = init_test_db();
    let err = project_change_status(&pool, ProjectChangeStatusReq {
        project_id: "ghost".to_string(),
        to_status: "PLANNED".to_string(),
        note: None,
        changed_by_person_id: None,
        if_match_updated_at: None,
    });
    assert_eq!(err.unwrap_err().code(), "NOT_FOUND");
}

// ══════════════════════════════════════════════════════════
//  project_update — 补充
// ══════════════════════════════════════════════════════════

#[test]
fn update_project_not_found() {
    let pool = init_test_db();
    let err = project_update(&pool, ProjectUpdateReq {
        id: "ghost".to_string(),
        name: Some("X".to_string()),
        description: None, priority: None, country_code: None,
        owner_person_id: None, start_date: None, due_date: None,
        tags: None, partner_id: None,
    });
    assert_eq!(err.unwrap_err().code(), "NOT_FOUND");
}

// ══════════════════════════════════════════════════════════
//  project_list — 分页
// ══════════════════════════════════════════════════════════

#[test]
fn list_projects_with_limit() {
    let pool = init_test_db();
    let ids = seed(&pool);
    for i in 0..5 {
        project_create(&pool, make_project_req(&ids, &format!("P{}", i))).unwrap();
    }

    let page = project_list(&pool, ProjectListReq {
        limit: Some(3),
        ..Default::default()
    }).unwrap();
    assert_eq!(page.items.len(), 3);
    assert_eq!(page.total, 5); // total should be unaffected by limit
}

#[test]
fn list_projects_with_offset() {
    let pool = init_test_db();
    let ids = seed(&pool);
    for i in 0..5 {
        project_create(&pool, make_project_req(&ids, &format!("O{}", i))).unwrap();
    }

    let all = project_list(&pool, ProjectListReq::default()).unwrap();
    let offset_page = project_list(&pool, ProjectListReq {
        offset: Some(2),
        ..Default::default()
    }).unwrap();
    assert_eq!(offset_page.items.len(), all.items.len() - 2);
    assert_eq!(offset_page.total, all.total); // total unchanged by offset
}

#[test]
fn list_projects_limit_clamped() {
    let pool = init_test_db();
    let ids = seed(&pool);
    project_create(&pool, make_project_req(&ids, "Clamp")).unwrap();

    // limit=0 → clamped to 1
    let page = project_list(&pool, ProjectListReq {
        limit: Some(0),
        ..Default::default()
    }).unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.limit, 1); // clamped

    // limit=300 → clamped to 200 (but only 1 project exists)
    let page = project_list(&pool, ProjectListReq {
        limit: Some(300),
        ..Default::default()
    }).unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.limit, 200); // clamped
}

// ══════════════════════════════════════════════════════════
//  project_get — 孤立 FK (owner/partner 被删除)
// ══════════════════════════════════════════════════════════

#[test]
fn get_project_missing_owner_shows_placeholder() {
    let pool = init_test_db();

    // Insert project with nonexistent owner via raw SQL (bypass FK at app level)
    {
        let conn = pool.0.lock().unwrap();
        // Disable FK checks temporarily
        conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, description, priority, current_status, country_code, partner_id, owner_person_id, created_at, updated_at, _version)
             VALUES ('orphan-proj', 'Orphan', '', 3, 'BACKLOG', 'US', 'missing-partner', 'missing-owner', datetime('now'), datetime('now'), 1)",
            [],
        ).unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    }

    let fetched = project_get(&pool, "orphan-proj").unwrap();
    assert_eq!(fetched.owner_name, "?");
    assert_eq!(fetched.partner_name, "?");
}

// ══════════════════════════════════════════════════════════
//  project_list — 服务端筛选 (statuses / countryCodes / partnerIds / ownerPersonIds / participantPersonIds / tags)
// ══════════════════════════════════════════════════════════

#[test]
fn list_filter_by_statuses() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let p1 = project_create(&pool, make_project_req(&ids, "Backlog1")).unwrap();
    let p2 = project_create(&pool, make_project_req(&ids, "Planned1")).unwrap();

    // Move p2 to PLANNED
    project_change_status(&pool, ProjectChangeStatusReq {
        project_id: p2.id.clone(),
        to_status: "PLANNED".to_string(),
        note: None, changed_by_person_id: None, if_match_updated_at: None,
    }).unwrap();

    // Filter by BACKLOG only
    let page = project_list(&pool, ProjectListReq {
        statuses: Some(vec!["BACKLOG".to_string()]),
        ..Default::default()
    }).unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].id, p1.id);

    // Filter by PLANNED only
    let page = project_list(&pool, ProjectListReq {
        statuses: Some(vec!["PLANNED".to_string()]),
        ..Default::default()
    }).unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].id, p2.id);

    // Filter by both
    let page = project_list(&pool, ProjectListReq {
        statuses: Some(vec!["BACKLOG".to_string(), "PLANNED".to_string()]),
        ..Default::default()
    }).unwrap();
    assert_eq!(page.total, 2);
}

#[test]
fn list_filter_by_country_codes() {
    let pool = init_test_db();
    let ids = seed(&pool);
    // make_project_req uses CN
    project_create(&pool, make_project_req(&ids, "CN-Project")).unwrap();

    // Create a US project
    project_create(&pool, ProjectCreateReq {
        name: "US-Project".to_string(),
        country_code: "US".to_string(),
        partner_id: ids.partner_id.clone(),
        owner_person_id: ids.person_id.clone(),
        description: None, priority: None, start_date: None, due_date: None,
        tags: None, created_by_person_id: None,
    }).unwrap();

    let page = project_list(&pool, ProjectListReq {
        country_codes: Some(vec!["US".to_string()]),
        ..Default::default()
    }).unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].name, "US-Project");
}

#[test]
fn list_filter_by_partner_ids() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let partner2 = partner_create(&pool, PartnerCreateReq {
        name: format!("Partner2-{}", uuid::Uuid::new_v4()),
        note: None,
    }).unwrap();

    project_create(&pool, make_project_req(&ids, "P-Partner1")).unwrap();
    project_create(&pool, ProjectCreateReq {
        name: "P-Partner2".to_string(),
        country_code: "CN".to_string(),
        partner_id: partner2.id.clone(),
        owner_person_id: ids.person_id.clone(),
        description: None, priority: None, start_date: None, due_date: None,
        tags: None, created_by_person_id: None,
    }).unwrap();

    let page = project_list(&pool, ProjectListReq {
        partner_ids: Some(vec![partner2.id.clone()]),
        ..Default::default()
    }).unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].name, "P-Partner2");
}

#[test]
fn list_filter_by_owner_person_ids() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let person2 = person_create(&pool, PersonCreateReq {
        display_name: "Owner2".to_string(),
        email: None, role: None, note: None,
    }).unwrap();

    project_create(&pool, make_project_req(&ids, "P-Owner1")).unwrap();
    project_create(&pool, ProjectCreateReq {
        name: "P-Owner2".to_string(),
        country_code: "CN".to_string(),
        partner_id: ids.partner_id.clone(),
        owner_person_id: person2.id.clone(),
        description: None, priority: None, start_date: None, due_date: None,
        tags: None, created_by_person_id: None,
    }).unwrap();

    let page = project_list(&pool, ProjectListReq {
        owner_person_ids: Some(vec![person2.id.clone()]),
        ..Default::default()
    }).unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].name, "P-Owner2");
}

#[test]
fn list_filter_by_participant_person_ids() {
    use app_lib::app::{assignment_add_member, AssignmentAddReq};

    let pool = init_test_db();
    let ids = seed(&pool);
    let person2 = person_create(&pool, PersonCreateReq {
        display_name: "Member2".to_string(),
        email: None, role: None, note: None,
    }).unwrap();

    let p1 = project_create(&pool, make_project_req(&ids, "P-Member")).unwrap();
    project_create(&pool, make_project_req(&ids, "P-NoMember")).unwrap();

    // Add person2 to p1 only
    assignment_add_member(&pool, AssignmentAddReq {
        project_id: p1.id.clone(),
        person_id: person2.id.clone(),
        role: None, start_at: None,
    }).unwrap();

    let page = project_list(&pool, ProjectListReq {
        participant_person_ids: Some(vec![person2.id.clone()]),
        ..Default::default()
    }).unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].id, p1.id);
}

#[test]
fn list_filter_by_tags() {
    let pool = init_test_db();
    let ids = seed(&pool);

    // make_project_req adds tag1, tag2
    project_create(&pool, make_project_req(&ids, "P-Tagged")).unwrap();

    // Project with different tags
    project_create(&pool, ProjectCreateReq {
        name: "P-UniqueTag".to_string(),
        country_code: "CN".to_string(),
        partner_id: ids.partner_id.clone(),
        owner_person_id: ids.person_id.clone(),
        tags: Some(vec!["special".to_string()]),
        description: None, priority: None, start_date: None, due_date: None,
        created_by_person_id: None,
    }).unwrap();

    let page = project_list(&pool, ProjectListReq {
        tags: Some(vec!["special".to_string()]),
        ..Default::default()
    }).unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].name, "P-UniqueTag");

    // Filter by tag1 should match first project
    let page = project_list(&pool, ProjectListReq {
        tags: Some(vec!["tag1".to_string()]),
        ..Default::default()
    }).unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].name, "P-Tagged");
}

#[test]
fn list_combined_filters() {
    let pool = init_test_db();
    let ids = seed(&pool);

    project_create(&pool, make_project_req(&ids, "CN-Backlog")).unwrap();
    project_create(&pool, ProjectCreateReq {
        name: "US-Backlog".to_string(),
        country_code: "US".to_string(),
        partner_id: ids.partner_id.clone(),
        owner_person_id: ids.person_id.clone(),
        description: None, priority: None, start_date: None, due_date: None,
        tags: None, created_by_person_id: None,
    }).unwrap();

    // Filter by CN + BACKLOG
    let page = project_list(&pool, ProjectListReq {
        country_codes: Some(vec!["CN".to_string()]),
        statuses: Some(vec!["BACKLOG".to_string()]),
        ..Default::default()
    }).unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].name, "CN-Backlog");
}

// ══════════════════════════════════════════════════════════
//  project_list — 排序
// ══════════════════════════════════════════════════════════

#[test]
fn list_sort_by_priority() {
    let pool = init_test_db();
    let ids = seed(&pool);

    project_create(&pool, ProjectCreateReq {
        name: "Low".to_string(),
        priority: Some(5),
        country_code: "CN".to_string(),
        partner_id: ids.partner_id.clone(),
        owner_person_id: ids.person_id.clone(),
        description: None, start_date: None, due_date: None,
        tags: None, created_by_person_id: None,
    }).unwrap();
    project_create(&pool, ProjectCreateReq {
        name: "High".to_string(),
        priority: Some(1),
        country_code: "CN".to_string(),
        partner_id: ids.partner_id.clone(),
        owner_person_id: ids.person_id.clone(),
        description: None, start_date: None, due_date: None,
        tags: None, created_by_person_id: None,
    }).unwrap();

    let page = project_list(&pool, ProjectListReq {
        sort_by: Some("priority".to_string()),
        sort_order: Some("asc".to_string()),
        ..Default::default()
    }).unwrap();
    assert_eq!(page.items[0].name, "High");
    assert_eq!(page.items[1].name, "Low");
}

#[test]
fn list_sort_by_due_date() {
    let pool = init_test_db();
    let ids = seed(&pool);

    project_create(&pool, ProjectCreateReq {
        name: "Later".to_string(),
        due_date: Some("2027-12-31".to_string()),
        country_code: "CN".to_string(),
        partner_id: ids.partner_id.clone(),
        owner_person_id: ids.person_id.clone(),
        description: None, priority: None, start_date: None,
        tags: None, created_by_person_id: None,
    }).unwrap();
    project_create(&pool, ProjectCreateReq {
        name: "Sooner".to_string(),
        due_date: Some("2026-06-01".to_string()),
        country_code: "CN".to_string(),
        partner_id: ids.partner_id.clone(),
        owner_person_id: ids.person_id.clone(),
        description: None, priority: None, start_date: None,
        tags: None, created_by_person_id: None,
    }).unwrap();
    project_create(&pool, ProjectCreateReq {
        name: "NoDue".to_string(),
        due_date: None,
        country_code: "CN".to_string(),
        partner_id: ids.partner_id.clone(),
        owner_person_id: ids.person_id.clone(),
        description: None, priority: None, start_date: None,
        tags: None, created_by_person_id: None,
    }).unwrap();

    let page = project_list(&pool, ProjectListReq {
        sort_by: Some("dueDate".to_string()),
        sort_order: Some("asc".to_string()),
        ..Default::default()
    }).unwrap();
    assert_eq!(page.items[0].name, "Sooner");
    assert_eq!(page.items[1].name, "Later");
    assert_eq!(page.items[2].name, "NoDue"); // NULL sorts last
}

// ══════════════════════════════════════════════════════════
//  project_list — Page 结构
// ══════════════════════════════════════════════════════════

#[test]
fn list_page_structure() {
    let pool = init_test_db();
    let ids = seed(&pool);
    for i in 0..5 {
        project_create(&pool, make_project_req(&ids, &format!("PG{}", i))).unwrap();
    }

    let page = project_list(&pool, ProjectListReq {
        limit: Some(2),
        offset: Some(1),
        ..Default::default()
    }).unwrap();
    assert_eq!(page.limit, 2);
    assert_eq!(page.offset, 1);
    assert_eq!(page.total, 5);
    assert_eq!(page.items.len(), 2);
}
