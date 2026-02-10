//! Export / Import JSON integration tests

use app_lib::app::{
    export_json_string, import_json_string, partner_create, person_create, project_create,
    project_list, assignment_add_member, project_change_status,
    PartnerCreateReq, PersonCreateReq, ProjectCreateReq, ProjectListReq,
    AssignmentAddReq, ProjectChangeStatusReq,
};
use app_lib::infra::db::init_test_db;

// ══════════════════════════════════════════════════════════
//  export_json_string
// ══════════════════════════════════════════════════════════

#[test]
fn export_empty_db_returns_valid_json() {
    let pool = init_test_db();
    let json_str = export_json_string(&pool, None).unwrap();
    let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    assert_eq!(json["schemaVersion"], 1);
    assert!(!json["exportedAt"].as_str().unwrap().is_empty());
    assert_eq!(json["persons"].as_array().unwrap().len(), 0);
    assert_eq!(json["partners"].as_array().unwrap().len(), 0);
    assert_eq!(json["projects"].as_array().unwrap().len(), 0);
    assert_eq!(json["assignments"].as_array().unwrap().len(), 0);
    assert_eq!(json["statusHistory"].as_array().unwrap().len(), 0);
}

#[test]
fn export_with_data_contains_all_entities() {
    let pool = init_test_db();

    // Seed data
    let owner = person_create(&pool, PersonCreateReq {
        display_name: "Alice".to_string(),
        email: Some("alice@test.com".to_string()),
        role: Some("PM".to_string()),
        note: None,
    }).unwrap();
    let member = person_create(&pool, PersonCreateReq {
        display_name: "Bob".to_string(),
        email: None, role: None, note: None,
    }).unwrap();
    let partner = partner_create(&pool, PartnerCreateReq {
        name: "TestCorp".to_string(),
        note: Some("A test partner".to_string()),
    }).unwrap();
    let project = project_create(&pool, ProjectCreateReq {
        name: "Export Test".to_string(),
        description: Some("Testing export".to_string()),
        priority: Some(4),
        country_code: "US".to_string(),
        partner_id: partner.id.clone(),
        owner_person_id: owner.id.clone(),
        start_date: Some("2026-01-01".to_string()),
        due_date: None,
        tags: Some(vec!["export".to_string(), "test".to_string()]),
        created_by_person_id: None,
    }).unwrap();

    // Add member assignment
    assignment_add_member(&pool, AssignmentAddReq {
        project_id: project.id.clone(),
        person_id: member.id.clone(),
        role: Some("developer".to_string()),
        start_at: None,
    }).unwrap();

    // Change status (creates additional status_history)
    project_change_status(&pool, ProjectChangeStatusReq {
        project_id: project.id.clone(),
        to_status: "PLANNED".to_string(),
        note: Some("Starting planning".to_string()),
        changed_by_person_id: Some(owner.id.clone()),
        if_match_updated_at: None,
    }).unwrap();

    let json_str = export_json_string(&pool, None).unwrap();
    let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // Verify counts
    assert_eq!(json["persons"].as_array().unwrap().len(), 2);
    assert_eq!(json["partners"].as_array().unwrap().len(), 1);
    assert_eq!(json["projects"].as_array().unwrap().len(), 1);
    // 2 assignments: owner (from create) + member
    assert_eq!(json["assignments"].as_array().unwrap().len(), 2);
    // 2 status_history: initial BACKLOG + PLANNED
    assert_eq!(json["statusHistory"].as_array().unwrap().len(), 2);

    // Verify project detail
    let proj = &json["projects"][0];
    assert_eq!(proj["name"], "Export Test");
    assert_eq!(proj["description"], "Testing export");
    assert_eq!(proj["priority"], 4);
    assert_eq!(proj["currentStatus"], "PLANNED");
    assert_eq!(proj["countryCode"], "US");
    assert_eq!(proj["partnerId"], partner.id);
    assert_eq!(proj["ownerPersonId"], owner.id);
    let tags = proj["tags"].as_array().unwrap();
    assert_eq!(tags.len(), 2);

    // Verify person detail
    let alice = json["persons"].as_array().unwrap().iter()
        .find(|p| p["displayName"] == "Alice")
        .unwrap();
    assert_eq!(alice["email"], "alice@test.com");
    assert_eq!(alice["role"], "PM");
    assert_eq!(alice["isActive"], true);

    // Verify partner detail
    let corp = &json["partners"][0];
    assert_eq!(corp["name"], "TestCorp");
    assert_eq!(corp["note"], "A test partner");
}

#[test]
fn export_json_is_pretty_printed() {
    let pool = init_test_db();
    let json_str = export_json_string(&pool, None).unwrap();
    // Pretty printed JSON should contain newlines and indentation
    assert!(json_str.contains('\n'));
    assert!(json_str.contains("  "));
}

#[test]
fn export_uses_camel_case_keys() {
    let pool = init_test_db();
    let owner = person_create(&pool, PersonCreateReq {
        display_name: "CamelCase".to_string(),
        email: None, role: None, note: None,
    }).unwrap();
    let partner = partner_create(&pool, PartnerCreateReq {
        name: "CamelPartner".to_string(),
        note: None,
    }).unwrap();
    project_create(&pool, ProjectCreateReq {
        name: "CamelProject".to_string(),
        description: None,
        priority: None,
        country_code: "US".to_string(),
        partner_id: partner.id,
        owner_person_id: owner.id,
        start_date: None,
        due_date: None,
        tags: None,
        created_by_person_id: None,
    }).unwrap();

    let json_str = export_json_string(&pool, None).unwrap();
    // camelCase keys
    assert!(json_str.contains("schemaVersion"));
    assert!(json_str.contains("exportedAt"));
    assert!(json_str.contains("displayName"));
    assert!(json_str.contains("isActive"));
    assert!(json_str.contains("createdAt"));
    assert!(json_str.contains("currentStatus"));
    assert!(json_str.contains("countryCode"));
    assert!(json_str.contains("ownerPersonId"));
    assert!(json_str.contains("partnerId"));
    // Should NOT contain snake_case
    assert!(!json_str.contains("schema_version"));
    assert!(!json_str.contains("display_name"));
    assert!(!json_str.contains("is_active"));
}

// ══════════════════════════════════════════════════════════
//  import_json_string
// ══════════════════════════════════════════════════════════

#[test]
fn import_into_empty_db_succeeds() {
    // Export from DB with data
    let pool1 = init_test_db();
    let owner = person_create(&pool1, PersonCreateReq {
        display_name: "Importer".to_string(),
        email: Some("imp@test.com".to_string()),
        role: Some("PM".to_string()),
        note: None,
    }).unwrap();
    let partner = partner_create(&pool1, PartnerCreateReq {
        name: "ImportPartner".to_string(),
        note: None,
    }).unwrap();
    project_create(&pool1, ProjectCreateReq {
        name: "ImportProject".to_string(),
        description: None, priority: None,
        country_code: "CN".to_string(),
        partner_id: partner.id.clone(),
        owner_person_id: owner.id.clone(),
        start_date: None, due_date: None,
        tags: Some(vec!["imported".to_string()]),
        created_by_person_id: None,
    }).unwrap();
    let json = export_json_string(&pool1, None).unwrap();

    // Import into fresh empty DB
    let pool2 = init_test_db();
    let result = import_json_string(&pool2, &json).unwrap();
    assert_eq!(result.persons, 1);
    assert_eq!(result.partners, 1);
    assert_eq!(result.projects, 1);
    assert!(result.assignments >= 1); // owner assignment
    assert!(result.status_history >= 1); // initial BACKLOG
    assert_eq!(result.skipped_duplicates, 0);

    // Verify data is queryable
    let page = project_list(&pool2, ProjectListReq::default()).unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].name, "ImportProject");
    assert!(page.items[0].tags.contains(&"imported".to_string()));
}

#[test]
fn import_duplicate_ids_are_skipped() {
    let pool = init_test_db();
    let owner = person_create(&pool, PersonCreateReq {
        display_name: "DupOwner".to_string(),
        email: None, role: None, note: None,
    }).unwrap();
    let partner = partner_create(&pool, PartnerCreateReq {
        name: "DupPartner".to_string(),
        note: None,
    }).unwrap();
    project_create(&pool, ProjectCreateReq {
        name: "DupProject".to_string(),
        description: None, priority: None,
        country_code: "US".to_string(),
        partner_id: partner.id.clone(),
        owner_person_id: owner.id.clone(),
        start_date: None, due_date: None, tags: None,
        created_by_person_id: None,
    }).unwrap();

    // Export and re-import into same DB
    let json = export_json_string(&pool, None).unwrap();
    let result = import_json_string(&pool, &json).unwrap();

    // All records should be skipped (same IDs)
    assert_eq!(result.persons, 0);
    assert_eq!(result.partners, 0);
    assert_eq!(result.projects, 0);
    assert_eq!(result.assignments, 0);
    assert_eq!(result.status_history, 0);
    assert!(result.skipped_duplicates > 0);
}

#[test]
fn import_invalid_json_returns_error() {
    let pool = init_test_db();
    let result = import_json_string(&pool, "not valid json {{{");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), "VALIDATION_ERROR");
}

#[test]
fn import_wrong_schema_version_returns_error() {
    let pool = init_test_db();
    let json = r#"{"schemaVersion":99,"exportedAt":"2026-01-01","persons":[],"partners":[],"projects":[],"assignments":[],"statusHistory":[]}"#;
    let result = import_json_string(&pool, json);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), "VALIDATION_ERROR");
}

#[test]
fn import_export_roundtrip_preserves_data() {
    let pool1 = init_test_db();
    let owner = person_create(&pool1, PersonCreateReq {
        display_name: "Roundtrip".to_string(),
        email: Some("rt@test.com".to_string()),
        role: Some("tester".to_string()),
        note: Some("test note".to_string()),
    }).unwrap();
    let partner = partner_create(&pool1, PartnerCreateReq {
        name: "RoundPartner".to_string(),
        note: Some("partner note".to_string()),
    }).unwrap();
    let proj = project_create(&pool1, ProjectCreateReq {
        name: "Roundtrip Project".to_string(),
        description: Some("desc".to_string()),
        priority: Some(2),
        country_code: "JP".to_string(),
        partner_id: partner.id.clone(),
        owner_person_id: owner.id.clone(),
        start_date: Some("2026-03-01".to_string()),
        due_date: Some("2026-12-31".to_string()),
        tags: Some(vec!["alpha".to_string(), "beta".to_string()]),
        created_by_person_id: None,
    }).unwrap();
    project_change_status(&pool1, ProjectChangeStatusReq {
        project_id: proj.id.clone(),
        to_status: "PLANNED".to_string(),
        note: Some("planning".to_string()),
        changed_by_person_id: Some(owner.id.clone()),
        if_match_updated_at: None,
    }).unwrap();

    let json = export_json_string(&pool1, None).unwrap();

    // Import into fresh DB
    let pool2 = init_test_db();
    import_json_string(&pool2, &json).unwrap();

    // Re-export and compare
    let json2 = export_json_string(&pool2, None).unwrap();
    let v1: serde_json::Value = serde_json::from_str(&json).unwrap();
    let v2: serde_json::Value = serde_json::from_str(&json2).unwrap();

    // Persons, partners, projects, assignments, status_history should match
    assert_eq!(v1["persons"], v2["persons"]);
    assert_eq!(v1["partners"], v2["partners"]);
    assert_eq!(v1["projects"], v2["projects"]);
    assert_eq!(v1["assignments"], v2["assignments"]);
    assert_eq!(v1["statusHistory"], v2["statusHistory"]);
}
