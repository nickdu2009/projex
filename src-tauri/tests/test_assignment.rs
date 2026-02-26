//! Assignment add/end member integration tests

use app_lib::app::{
    assignment_add_member, assignment_end_member, assignment_list_by_project, partner_create,
    person_create, project_create, project_get, AssignmentAddReq, AssignmentEndReq,
    PartnerCreateReq, PersonCreateReq, ProjectCreateReq,
};
use app_lib::infra::db::init_test_db;

// ──────────────────────── Helper ────────────────────────

#[allow(dead_code)]
struct TestSeedIds {
    owner_id: String,
    partner_id: String,
    project_id: String,
}

fn seed(pool: &app_lib::infra::DbPool) -> TestSeedIds {
    let owner = person_create(
        pool,
        PersonCreateReq {
            display_name: "Owner".to_string(),
            email: None,
            role: None,
            note: None,
        },
    )
    .unwrap();
    let partner = partner_create(
        pool,
        PartnerCreateReq {
            name: format!("P-{}", uuid::Uuid::new_v4()),
            note: None,
        },
    )
    .unwrap();
    let project = project_create(
        pool,
        ProjectCreateReq {
            name: "Test Project".to_string(),
            description: None,
            priority: None,
            country_code: "US".to_string(),
            partner_id: partner.id.clone(),
            owner_person_id: owner.id.clone(),
            product_name: None,
            start_date: None,
            due_date: None,
            tags: None,
            created_by_person_id: None,
        },
    )
    .unwrap();
    TestSeedIds {
        owner_id: owner.id,
        partner_id: partner.id,
        project_id: project.id,
    }
}

// ══════════════════════════════════════════════════════════
//  assignment_add_member
// ══════════════════════════════════════════════════════════

#[test]
fn add_member_succeeds() {
    let pool = init_test_db();
    let ids = seed(&pool);

    let member = person_create(
        &pool,
        PersonCreateReq {
            display_name: "Member".to_string(),
            email: None,
            role: None,
            note: None,
        },
    )
    .unwrap();

    assignment_add_member(
        &pool,
        AssignmentAddReq {
            project_id: ids.project_id.clone(),
            person_id: member.id.clone(),
            role: Some("developer".to_string()),
            start_at: None,
        },
    )
    .unwrap();

    let proj = project_get(&pool, &ids.project_id).unwrap();
    let member_asgn = proj
        .assignments
        .iter()
        .find(|a| a.person_id == member.id)
        .unwrap();
    assert_eq!(member_asgn.role, "developer");
    assert!(member_asgn.end_at.is_none());
}

#[test]
fn add_member_default_role_is_member() {
    let pool = init_test_db();
    let ids = seed(&pool);

    let member = person_create(
        &pool,
        PersonCreateReq {
            display_name: "Default Role".to_string(),
            email: None,
            role: None,
            note: None,
        },
    )
    .unwrap();

    assignment_add_member(
        &pool,
        AssignmentAddReq {
            project_id: ids.project_id.clone(),
            person_id: member.id.clone(),
            role: None, // default
            start_at: None,
        },
    )
    .unwrap();

    let proj = project_get(&pool, &ids.project_id).unwrap();
    let asgn = proj
        .assignments
        .iter()
        .find(|a| a.person_id == member.id)
        .unwrap();
    assert_eq!(asgn.role, "member");
}

#[test]
fn add_member_duplicate_active_fails() {
    let pool = init_test_db();
    let ids = seed(&pool);

    let member = person_create(
        &pool,
        PersonCreateReq {
            display_name: "Dup".to_string(),
            email: None,
            role: None,
            note: None,
        },
    )
    .unwrap();

    assignment_add_member(
        &pool,
        AssignmentAddReq {
            project_id: ids.project_id.clone(),
            person_id: member.id.clone(),
            role: None,
            start_at: None,
        },
    )
    .unwrap();

    // Adding again should fail
    let err = assignment_add_member(
        &pool,
        AssignmentAddReq {
            project_id: ids.project_id.clone(),
            person_id: member.id.clone(),
            role: None,
            start_at: None,
        },
    );
    assert_eq!(err.unwrap_err().code(), "ASSIGNMENT_ALREADY_ACTIVE");
}

#[test]
fn add_member_owner_already_active_fails() {
    let pool = init_test_db();
    let ids = seed(&pool);

    // Owner already has an active assignment from project_create
    let err = assignment_add_member(
        &pool,
        AssignmentAddReq {
            project_id: ids.project_id.clone(),
            person_id: ids.owner_id.clone(),
            role: Some("developer".to_string()),
            start_at: None,
        },
    );
    assert_eq!(err.unwrap_err().code(), "ASSIGNMENT_ALREADY_ACTIVE");
}

// ══════════════════════════════════════════════════════════
//  assignment_end_member
// ══════════════════════════════════════════════════════════

#[test]
fn end_member_succeeds() {
    let pool = init_test_db();
    let ids = seed(&pool);

    let member = person_create(
        &pool,
        PersonCreateReq {
            display_name: "End Me".to_string(),
            email: None,
            role: None,
            note: None,
        },
    )
    .unwrap();

    assignment_add_member(
        &pool,
        AssignmentAddReq {
            project_id: ids.project_id.clone(),
            person_id: member.id.clone(),
            role: None,
            start_at: None,
        },
    )
    .unwrap();

    assignment_end_member(
        &pool,
        AssignmentEndReq {
            project_id: ids.project_id.clone(),
            person_id: member.id.clone(),
            end_at: None,
        },
    )
    .unwrap();

    let proj = project_get(&pool, &ids.project_id).unwrap();
    let asgn = proj
        .assignments
        .iter()
        .find(|a| a.person_id == member.id)
        .unwrap();
    assert!(asgn.end_at.is_some());
}

#[test]
fn end_member_no_active_fails() {
    let pool = init_test_db();
    let ids = seed(&pool);

    let member = person_create(
        &pool,
        PersonCreateReq {
            display_name: "Not Assigned".to_string(),
            email: None,
            role: None,
            note: None,
        },
    )
    .unwrap();

    let err = assignment_end_member(
        &pool,
        AssignmentEndReq {
            project_id: ids.project_id.clone(),
            person_id: member.id.clone(),
            end_at: None,
        },
    );
    assert_eq!(err.unwrap_err().code(), "ASSIGNMENT_NOT_ACTIVE");
}

#[test]
fn end_member_then_readd_succeeds() {
    let pool = init_test_db();
    let ids = seed(&pool);

    let member = person_create(
        &pool,
        PersonCreateReq {
            display_name: "ReAdd".to_string(),
            email: None,
            role: None,
            note: None,
        },
    )
    .unwrap();

    // Add
    assignment_add_member(
        &pool,
        AssignmentAddReq {
            project_id: ids.project_id.clone(),
            person_id: member.id.clone(),
            role: Some("developer".to_string()),
            start_at: None,
        },
    )
    .unwrap();

    // End
    assignment_end_member(
        &pool,
        AssignmentEndReq {
            project_id: ids.project_id.clone(),
            person_id: member.id.clone(),
            end_at: None,
        },
    )
    .unwrap();

    // Re-add should work
    assignment_add_member(
        &pool,
        AssignmentAddReq {
            project_id: ids.project_id.clone(),
            person_id: member.id.clone(),
            role: Some("lead".to_string()),
            start_at: None,
        },
    )
    .unwrap();

    let proj = project_get(&pool, &ids.project_id).unwrap();
    let active_asgns: Vec<_> = proj
        .assignments
        .iter()
        .filter(|a| a.person_id == member.id && a.end_at.is_none())
        .collect();
    assert_eq!(active_asgns.len(), 1);
    assert_eq!(active_asgns[0].role, "lead");
}

// ══════════════════════════════════════════════════════════
//  补充: 自定义 start_at / end_at
// ══════════════════════════════════════════════════════════

#[test]
fn add_member_custom_start_at() {
    let pool = init_test_db();
    let ids = seed(&pool);

    let member = person_create(
        &pool,
        PersonCreateReq {
            display_name: "CustomStart".to_string(),
            email: None,
            role: None,
            note: None,
        },
    )
    .unwrap();

    assignment_add_member(
        &pool,
        AssignmentAddReq {
            project_id: ids.project_id.clone(),
            person_id: member.id.clone(),
            role: None,
            start_at: Some("2025-06-15T00:00:00Z".to_string()),
        },
    )
    .unwrap();

    let proj = project_get(&pool, &ids.project_id).unwrap();
    let asgn = proj
        .assignments
        .iter()
        .find(|a| a.person_id == member.id)
        .unwrap();
    assert_eq!(asgn.start_at, "2025-06-15T00:00:00Z");
}

#[test]
fn end_member_custom_end_at() {
    let pool = init_test_db();
    let ids = seed(&pool);

    let member = person_create(
        &pool,
        PersonCreateReq {
            display_name: "CustomEnd".to_string(),
            email: None,
            role: None,
            note: None,
        },
    )
    .unwrap();

    assignment_add_member(
        &pool,
        AssignmentAddReq {
            project_id: ids.project_id.clone(),
            person_id: member.id.clone(),
            role: None,
            start_at: None,
        },
    )
    .unwrap();

    assignment_end_member(
        &pool,
        AssignmentEndReq {
            project_id: ids.project_id.clone(),
            person_id: member.id.clone(),
            end_at: Some("2026-12-31T23:59:59Z".to_string()),
        },
    )
    .unwrap();

    let proj = project_get(&pool, &ids.project_id).unwrap();
    let asgn = proj
        .assignments
        .iter()
        .find(|a| a.person_id == member.id)
        .unwrap();
    assert_eq!(asgn.end_at, Some("2026-12-31T23:59:59Z".to_string()));
}

// ══════════════════════════════════════════════════════════
//  assignment_list_by_project
// ══════════════════════════════════════════════════════════

#[test]
fn list_by_project_returns_all_assignments() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let member = person_create(
        &pool,
        PersonCreateReq {
            display_name: "ListMember".to_string(),
            email: None,
            role: None,
            note: None,
        },
    )
    .unwrap();

    assignment_add_member(
        &pool,
        AssignmentAddReq {
            project_id: ids.project_id.clone(),
            person_id: member.id.clone(),
            role: Some("tester".to_string()),
            start_at: None,
        },
    )
    .unwrap();

    let list = assignment_list_by_project(&pool, &ids.project_id).unwrap();
    // Owner (auto-created) + new member
    assert_eq!(list.len(), 2);
    assert!(list.iter().any(|a| a.person_id == ids.owner_id));
    assert!(list
        .iter()
        .any(|a| a.person_id == member.id && a.role == "tester"));
}

#[test]
fn list_by_project_includes_person_name() {
    let pool = init_test_db();
    let ids = seed(&pool);

    let list = assignment_list_by_project(&pool, &ids.project_id).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].person_name, "Owner"); // name from seed()
}

#[test]
fn list_by_project_empty_for_unknown_project() {
    let pool = init_test_db();
    let list = assignment_list_by_project(&pool, "nonexistent").unwrap();
    assert!(list.is_empty());
}

#[test]
fn list_by_project_active_before_ended() {
    let pool = init_test_db();
    let ids = seed(&pool);
    let member = person_create(
        &pool,
        PersonCreateReq {
            display_name: "EndedMember".to_string(),
            email: None,
            role: None,
            note: None,
        },
    )
    .unwrap();

    assignment_add_member(
        &pool,
        AssignmentAddReq {
            project_id: ids.project_id.clone(),
            person_id: member.id.clone(),
            role: None,
            start_at: None,
        },
    )
    .unwrap();
    assignment_end_member(
        &pool,
        AssignmentEndReq {
            project_id: ids.project_id.clone(),
            person_id: member.id.clone(),
            end_at: None,
        },
    )
    .unwrap();

    let list = assignment_list_by_project(&pool, &ids.project_id).unwrap();
    // Owner (active, end_at=NULL) should come before ended member
    assert_eq!(list[0].person_id, ids.owner_id);
    assert!(list[0].end_at.is_none());
    assert_eq!(list[1].person_id, member.id);
    assert!(list[1].end_at.is_some());
}
