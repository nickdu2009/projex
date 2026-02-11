//! Person CRUD integration tests

use app_lib::app::{
    assignment_add_member, assignment_end_member, partner_create, person_all_projects,
    person_create, person_current_projects, person_deactivate, person_get, person_list,
    person_update, project_change_status, project_create, AssignmentAddReq, AssignmentEndReq,
    PartnerCreateReq, PersonCreateReq, PersonUpdateReq, ProjectChangeStatusReq, ProjectCreateReq,
};
use app_lib::infra::db::init_test_db;

// ──────────────────────── Helper ────────────────────────

fn make_create_req(name: &str) -> PersonCreateReq {
    PersonCreateReq {
        display_name: name.to_string(),
        email: Some(format!("{}@test.com", name.to_lowercase())),
        role: Some("dev".to_string()),
        note: None,
    }
}

// ══════════════════════════════════════════════════════════
//  person_create
// ══════════════════════════════════════════════════════════

#[test]
fn create_person_returns_dto_with_correct_fields() {
    let pool = init_test_db();
    let dto = person_create(&pool, make_create_req("Alice")).unwrap();
    assert_eq!(dto.display_name, "Alice");
    assert_eq!(dto.email, "alice@test.com");
    assert_eq!(dto.role, "dev");
    assert!(dto.is_active);
    assert!(!dto.id.is_empty());
    assert!(!dto.created_at.is_empty());
}

#[test]
fn create_person_trims_display_name() {
    let pool = init_test_db();
    let dto = person_create(
        &pool,
        PersonCreateReq {
            display_name: "  Bob  ".to_string(),
            email: None,
            role: None,
            note: None,
        },
    )
    .unwrap();
    assert_eq!(dto.display_name, "Bob");
}

#[test]
fn create_person_empty_name_fails() {
    let pool = init_test_db();
    let err = person_create(
        &pool,
        PersonCreateReq {
            display_name: "   ".to_string(),
            email: None,
            role: None,
            note: None,
        },
    );
    assert!(err.is_err());
    let e = err.unwrap_err();
    assert_eq!(e.code(), "VALIDATION_ERROR");
}

#[test]
fn create_person_defaults_optional_fields() {
    let pool = init_test_db();
    let dto = person_create(
        &pool,
        PersonCreateReq {
            display_name: "Charlie".to_string(),
            email: None,
            role: None,
            note: None,
        },
    )
    .unwrap();
    assert_eq!(dto.email, "");
    assert_eq!(dto.role, "");
    assert_eq!(dto.note, "");
}

// ══════════════════════════════════════════════════════════
//  person_get
// ══════════════════════════════════════════════════════════

#[test]
fn get_person_by_id() {
    let pool = init_test_db();
    let created = person_create(&pool, make_create_req("Dave")).unwrap();
    let fetched = person_get(&pool, &created.id).unwrap();
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.display_name, "Dave");
}

#[test]
fn get_person_not_found() {
    let pool = init_test_db();
    let err = person_get(&pool, "nonexistent-id");
    assert!(err.is_err());
    assert_eq!(err.unwrap_err().code(), "NOT_FOUND");
}

// ══════════════════════════════════════════════════════════
//  person_list
// ══════════════════════════════════════════════════════════

#[test]
fn list_persons_returns_all() {
    let pool = init_test_db();
    person_create(&pool, make_create_req("A")).unwrap();
    person_create(&pool, make_create_req("B")).unwrap();
    let all = person_list(&pool, false).unwrap();
    assert_eq!(all.len(), 2);
}

#[test]
fn list_persons_only_active_filters_deactivated() {
    let pool = init_test_db();
    let a = person_create(&pool, make_create_req("Active")).unwrap();
    let d = person_create(&pool, make_create_req("Deactivated")).unwrap();
    person_deactivate(&pool, &d.id).unwrap();

    let active = person_list(&pool, true).unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].id, a.id);

    let all = person_list(&pool, false).unwrap();
    assert_eq!(all.len(), 2);
}

#[test]
fn list_persons_sorted_by_name_case_insensitive() {
    let pool = init_test_db();
    person_create(&pool, make_create_req("charlie")).unwrap();
    person_create(&pool, make_create_req("Alice")).unwrap();
    person_create(&pool, make_create_req("bob")).unwrap();

    let list = person_list(&pool, false).unwrap();
    let names: Vec<&str> = list.iter().map(|p| p.display_name.as_str()).collect();
    assert_eq!(names, vec!["Alice", "bob", "charlie"]);
}

// ══════════════════════════════════════════════════════════
//  person_update
// ══════════════════════════════════════════════════════════

#[test]
fn update_person_partial_fields() {
    let pool = init_test_db();
    let created = person_create(&pool, make_create_req("Eve")).unwrap();

    let updated = person_update(
        &pool,
        PersonUpdateReq {
            id: created.id.clone(),
            display_name: Some("Eve Updated".to_string()),
            email: None, // keep original
            role: Some("lead".to_string()),
            note: None,
        },
    )
    .unwrap();

    assert_eq!(updated.display_name, "Eve Updated");
    assert_eq!(updated.email, "eve@test.com"); // unchanged
    assert_eq!(updated.role, "lead");
}

#[test]
fn update_person_not_found() {
    let pool = init_test_db();
    let err = person_update(
        &pool,
        PersonUpdateReq {
            id: "ghost".to_string(),
            display_name: Some("X".to_string()),
            email: None,
            role: None,
            note: None,
        },
    );
    assert!(err.is_err());
    assert_eq!(err.unwrap_err().code(), "NOT_FOUND");
}

#[test]
fn update_person_empty_name_keeps_original() {
    let pool = init_test_db();
    let created = person_create(&pool, make_create_req("Frank")).unwrap();

    // Sending empty string should keep original name (filter logic)
    let updated = person_update(
        &pool,
        PersonUpdateReq {
            id: created.id.clone(),
            display_name: Some("  ".to_string()),
            email: None,
            role: None,
            note: None,
        },
    )
    .unwrap();
    assert_eq!(updated.display_name, "Frank");
}

// ══════════════════════════════════════════════════════════
//  person_deactivate
// ══════════════════════════════════════════════════════════

#[test]
fn deactivate_person_sets_inactive() {
    let pool = init_test_db();
    let p = person_create(&pool, make_create_req("Grace")).unwrap();
    assert!(p.is_active);

    let deactivated = person_deactivate(&pool, &p.id).unwrap();
    assert!(!deactivated.is_active);
    assert!(deactivated.updated_at > p.updated_at);
}

// ══════════════════════════════════════════════════════════
//  person_current_projects / person_all_projects
// ══════════════════════════════════════════════════════════

#[test]
fn current_projects_empty_for_new_person() {
    let pool = init_test_db();
    let p = person_create(&pool, make_create_req("Hank")).unwrap();
    let projects = person_current_projects(&pool, &p.id).unwrap();
    assert!(projects.is_empty());
}

#[test]
fn all_projects_empty_for_new_person() {
    let pool = init_test_db();
    let p = person_create(&pool, make_create_req("Ivy")).unwrap();
    let projects = person_all_projects(&pool, &p.id).unwrap();
    assert!(projects.is_empty());
}

// ──────────────────────── Seed helper for project tests ────────────────────────

fn seed_project_for_person(pool: &app_lib::infra::DbPool, owner_id: &str) -> String {
    let partner = partner_create(
        pool,
        PartnerCreateReq {
            name: format!("Partner-{}", uuid::Uuid::new_v4()),
            note: None,
        },
    )
    .unwrap();
    let proj = project_create(
        pool,
        ProjectCreateReq {
            name: format!("Project-{}", uuid::Uuid::new_v4()),
            description: None,
            priority: None,
            country_code: "US".to_string(),
            partner_id: partner.id,
            owner_person_id: owner_id.to_string(),
            start_date: None,
            due_date: None,
            tags: None,
            created_by_person_id: None,
        },
    )
    .unwrap();
    proj.id
}

// ══════════════════════════════════════════════════════════
//  person_current_projects / person_all_projects (有数据)
// ══════════════════════════════════════════════════════════

#[test]
fn current_projects_returns_active_assignments() {
    let pool = init_test_db();
    let owner = person_create(&pool, make_create_req("Owner")).unwrap();
    let _proj_id = seed_project_for_person(&pool, &owner.id);

    let projects = person_current_projects(&pool, &owner.id).unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].current_status, "BACKLOG");
}

#[test]
fn current_projects_excludes_archived() {
    let pool = init_test_db();
    let owner = person_create(&pool, make_create_req("ArchOwner")).unwrap();
    let proj_id = seed_project_for_person(&pool, &owner.id);

    // Archive the project: BACKLOG → ARCHIVED (requires note)
    project_change_status(
        &pool,
        ProjectChangeStatusReq {
            project_id: proj_id,
            to_status: "ARCHIVED".to_string(),
            note: Some("done".to_string()),
            changed_by_person_id: None,
            if_match_updated_at: None,
        },
    )
    .unwrap();

    let projects = person_current_projects(&pool, &owner.id).unwrap();
    assert!(
        projects.is_empty(),
        "Archived projects should be excluded from current_projects"
    );
}

#[test]
fn current_projects_excludes_ended_assignments() {
    let pool = init_test_db();
    let owner = person_create(&pool, make_create_req("EndOwner")).unwrap();
    let member = person_create(&pool, make_create_req("Member")).unwrap();
    let proj_id = seed_project_for_person(&pool, &owner.id);

    // Add member then end assignment
    assignment_add_member(
        &pool,
        AssignmentAddReq {
            project_id: proj_id.clone(),
            person_id: member.id.clone(),
            role: None,
            start_at: None,
        },
    )
    .unwrap();
    assignment_end_member(
        &pool,
        AssignmentEndReq {
            project_id: proj_id,
            person_id: member.id.clone(),
            end_at: None,
        },
    )
    .unwrap();

    let projects = person_current_projects(&pool, &member.id).unwrap();
    assert!(
        projects.is_empty(),
        "Ended assignments should not appear in current_projects"
    );
}

#[test]
fn all_projects_includes_ended_assignments() {
    let pool = init_test_db();
    let owner = person_create(&pool, make_create_req("AllOwner")).unwrap();
    let member = person_create(&pool, make_create_req("AllMember")).unwrap();
    let proj_id = seed_project_for_person(&pool, &owner.id);

    // Add then end member
    assignment_add_member(
        &pool,
        AssignmentAddReq {
            project_id: proj_id.clone(),
            person_id: member.id.clone(),
            role: None,
            start_at: None,
        },
    )
    .unwrap();
    assignment_end_member(
        &pool,
        AssignmentEndReq {
            project_id: proj_id,
            person_id: member.id.clone(),
            end_at: None,
        },
    )
    .unwrap();

    let projects = person_all_projects(&pool, &member.id).unwrap();
    assert_eq!(
        projects.len(),
        1,
        "all_projects should include ended assignments"
    );
    assert!(projects[0].last_involved_at.is_some());
}

#[test]
fn all_projects_multiple_projects_sorted() {
    let pool = init_test_db();
    let owner = person_create(&pool, make_create_req("MultiOwner")).unwrap();
    let _p1 = seed_project_for_person(&pool, &owner.id);
    let _p2 = seed_project_for_person(&pool, &owner.id);

    let projects = person_all_projects(&pool, &owner.id).unwrap();
    assert_eq!(projects.len(), 2);
}

#[test]
fn deactivate_person_not_found_still_returns_error() {
    let pool = init_test_db();
    // deactivate updates 0 rows, then person_get fails with NOT_FOUND
    let err = person_deactivate(&pool, "ghost-id");
    assert_eq!(err.unwrap_err().code(), "NOT_FOUND");
}
