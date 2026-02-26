//! Partner CRUD integration tests

use app_lib::app::{
    partner_create, partner_deactivate, partner_get, partner_list, partner_projects,
    partner_update, person_create, project_create, PartnerCreateReq, PartnerUpdateReq,
    PersonCreateReq, ProjectCreateReq,
};
use app_lib::infra::db::init_test_db;

// ──────────────────────── Helper ────────────────────────

fn make_create_req(name: &str) -> PartnerCreateReq {
    PartnerCreateReq {
        name: name.to_string(),
        note: Some("test note".to_string()),
    }
}

// ══════════════════════════════════════════════════════════
//  partner_create
// ══════════════════════════════════════════════════════════

#[test]
fn create_partner_returns_correct_fields() {
    let pool = init_test_db();
    let dto = partner_create(&pool, make_create_req("Acme Corp")).unwrap();
    assert_eq!(dto.name, "Acme Corp");
    assert_eq!(dto.note, "test note");
    assert!(dto.is_active);
    assert!(!dto.id.is_empty());
}

#[test]
fn create_partner_trims_name() {
    let pool = init_test_db();
    let dto = partner_create(
        &pool,
        PartnerCreateReq {
            name: "  Trimmed  ".to_string(),
            note: None,
        },
    )
    .unwrap();
    assert_eq!(dto.name, "Trimmed");
}

#[test]
fn create_partner_empty_name_fails() {
    let pool = init_test_db();
    let err = partner_create(
        &pool,
        PartnerCreateReq {
            name: "  ".to_string(),
            note: None,
        },
    );
    assert!(err.is_err());
    assert_eq!(err.unwrap_err().code(), "VALIDATION_ERROR");
}

#[test]
fn create_partner_defaults_note() {
    let pool = init_test_db();
    let dto = partner_create(
        &pool,
        PartnerCreateReq {
            name: "NoNote".to_string(),
            note: None,
        },
    )
    .unwrap();
    assert_eq!(dto.note, "");
}

#[test]
fn create_partner_unique_name_constraint() {
    let pool = init_test_db();
    partner_create(&pool, make_create_req("UniquePartner")).unwrap();
    // partners table has UNIQUE INDEX on name
    let err = partner_create(&pool, make_create_req("UniquePartner"));
    assert!(err.is_err());
    assert_eq!(err.unwrap_err().code(), "DB_ERROR");
}

// ══════════════════════════════════════════════════════════
//  partner_get
// ══════════════════════════════════════════════════════════

#[test]
fn get_partner_by_id() {
    let pool = init_test_db();
    let created = partner_create(&pool, make_create_req("GetTest")).unwrap();
    let fetched = partner_get(&pool, &created.id).unwrap();
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.name, "GetTest");
}

#[test]
fn get_partner_not_found() {
    let pool = init_test_db();
    let err = partner_get(&pool, "nonexistent");
    assert!(err.is_err());
    assert_eq!(err.unwrap_err().code(), "NOT_FOUND");
}

// ══════════════════════════════════════════════════════════
//  partner_list
// ══════════════════════════════════════════════════════════

#[test]
fn list_partners_returns_all() {
    let pool = init_test_db();
    partner_create(&pool, make_create_req("Alpha")).unwrap();
    partner_create(&pool, make_create_req("Beta")).unwrap();
    let all = partner_list(&pool, false).unwrap();
    assert_eq!(all.len(), 2);
}

#[test]
fn list_partners_only_active_filters() {
    let pool = init_test_db();
    let a = partner_create(&pool, make_create_req("Active Inc")).unwrap();
    let d = partner_create(&pool, make_create_req("Defunct LLC")).unwrap();
    partner_deactivate(&pool, &d.id).unwrap();

    let active = partner_list(&pool, true).unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].id, a.id);
}

#[test]
fn list_partners_sorted_by_name_case_insensitive() {
    let pool = init_test_db();
    partner_create(&pool, make_create_req("charlie corp")).unwrap();
    partner_create(&pool, make_create_req("Alpha Inc")).unwrap();
    partner_create(&pool, make_create_req("beta LLC")).unwrap();

    let list = partner_list(&pool, false).unwrap();
    let names: Vec<&str> = list.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["Alpha Inc", "beta LLC", "charlie corp"]);
}

// ══════════════════════════════════════════════════════════
//  partner_update
// ══════════════════════════════════════════════════════════

#[test]
fn update_partner_partial_fields() {
    let pool = init_test_db();
    let created = partner_create(&pool, make_create_req("UpdateMe")).unwrap();

    let updated = partner_update(
        &pool,
        PartnerUpdateReq {
            id: created.id.clone(),
            name: Some("Updated Name".to_string()),
            note: None, // keep original
        },
    )
    .unwrap();

    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.note, "test note"); // unchanged
}

#[test]
fn update_partner_not_found() {
    let pool = init_test_db();
    let err = partner_update(
        &pool,
        PartnerUpdateReq {
            id: "ghost".to_string(),
            name: Some("X".to_string()),
            note: None,
        },
    );
    assert!(err.is_err());
    assert_eq!(err.unwrap_err().code(), "NOT_FOUND");
}

#[test]
fn update_partner_empty_name_keeps_original() {
    let pool = init_test_db();
    let created = partner_create(&pool, make_create_req("KeepName")).unwrap();

    let updated = partner_update(
        &pool,
        PartnerUpdateReq {
            id: created.id.clone(),
            name: Some("  ".to_string()),
            note: None,
        },
    )
    .unwrap();
    assert_eq!(updated.name, "KeepName");
}

// ══════════════════════════════════════════════════════════
//  partner_deactivate
// ══════════════════════════════════════════════════════════

#[test]
fn deactivate_partner_sets_inactive() {
    let pool = init_test_db();
    let p = partner_create(&pool, make_create_req("Deact Corp")).unwrap();
    assert!(p.is_active);

    let deactivated = partner_deactivate(&pool, &p.id).unwrap();
    assert!(!deactivated.is_active);
}

// ══════════════════════════════════════════════════════════
//  partner_projects
// ══════════════════════════════════════════════════════════

#[test]
fn partner_projects_empty_initially() {
    let pool = init_test_db();
    let p = partner_create(&pool, make_create_req("Empty Projects")).unwrap();
    let projects = partner_projects(&pool, &p.id).unwrap();
    assert!(projects.is_empty());
}

#[test]
fn partner_projects_returns_associated_projects() {
    let pool = init_test_db();
    let partner = partner_create(&pool, make_create_req("HasProjects")).unwrap();
    let owner = person_create(
        &pool,
        PersonCreateReq {
            display_name: "Owner".to_string(),
            email: None,
            role: None,
            note: None,
        },
    )
    .unwrap();

    project_create(
        &pool,
        ProjectCreateReq {
            name: "Proj A".to_string(),
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
    project_create(
        &pool,
        ProjectCreateReq {
            name: "Proj B".to_string(),
            description: None,
            priority: None,
            country_code: "CN".to_string(),
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

    let projects = partner_projects(&pool, &partner.id).unwrap();
    assert_eq!(projects.len(), 2);
    // Ordered by updated_at DESC — both created nearly simultaneously
    let names: Vec<&str> = projects.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"Proj A"));
    assert!(names.contains(&"Proj B"));
}

#[test]
fn partner_projects_does_not_include_other_partners() {
    let pool = init_test_db();
    let partner_a = partner_create(&pool, make_create_req("PartnerA")).unwrap();
    let partner_b = partner_create(&pool, make_create_req("PartnerB")).unwrap();
    let owner = person_create(
        &pool,
        PersonCreateReq {
            display_name: "Owner".to_string(),
            email: None,
            role: None,
            note: None,
        },
    )
    .unwrap();

    project_create(
        &pool,
        ProjectCreateReq {
            name: "A's Project".to_string(),
            description: None,
            priority: None,
            country_code: "US".to_string(),
            partner_id: partner_a.id.clone(),
            owner_person_id: owner.id.clone(),
            product_name: None,
            start_date: None,
            due_date: None,
            tags: None,
            created_by_person_id: None,
        },
    )
    .unwrap();
    project_create(
        &pool,
        ProjectCreateReq {
            name: "B's Project".to_string(),
            description: None,
            priority: None,
            country_code: "US".to_string(),
            partner_id: partner_b.id.clone(),
            owner_person_id: owner.id.clone(),
            product_name: None,
            start_date: None,
            due_date: None,
            tags: None,
            created_by_person_id: None,
        },
    )
    .unwrap();

    let a_projects = partner_projects(&pool, &partner_a.id).unwrap();
    assert_eq!(a_projects.len(), 1);
    assert_eq!(a_projects[0].name, "A's Project");
}

#[test]
fn deactivate_partner_not_found_still_returns_error() {
    let pool = init_test_db();
    let err = partner_deactivate(&pool, "ghost-partner");
    assert_eq!(err.unwrap_err().code(), "NOT_FOUND");
}
