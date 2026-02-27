//! Person CSV export / import integration tests

use app_lib::app::{export_persons_csv, import_persons_csv, person_create, person_get, PersonCreateReq};
use app_lib::infra::db::init_test_db;

// ══════════════════════════════════════════════════════════
//  export_persons_csv
// ══════════════════════════════════════════════════════════

#[test]
fn export_empty_db_returns_header_only() {
    let pool = init_test_db();
    let csv = export_persons_csv(&pool).unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0], "display_name,email,role,note,is_active");
}

#[test]
fn export_persons_returns_correct_columns() {
    let pool = init_test_db();
    person_create(
        &pool,
        PersonCreateReq {
            display_name: "Alice".to_string(),
            email: Some("alice@test.com".to_string()),
            role: Some("backend_developer".to_string()),
            note: Some("Senior dev".to_string()),
        },
    )
    .unwrap();

    let csv = export_persons_csv(&pool).unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 2); // header + 1 row
    assert_eq!(lines[0], "display_name,email,role,note,is_active");
    assert!(lines[1].contains("Alice"));
    assert!(lines[1].contains("alice@test.com"));
    assert!(lines[1].contains("backend_developer"));
    assert!(lines[1].contains("Senior dev"));
    assert!(lines[1].contains("true"));
}

#[test]
fn export_persons_sorted_by_name_case_insensitive() {
    let pool = init_test_db();
    for name in &["charlie", "Alice", "bob"] {
        person_create(
            &pool,
            PersonCreateReq {
                display_name: name.to_string(),
                email: None,
                role: None,
                note: None,
            },
        )
        .unwrap();
    }

    let csv = export_persons_csv(&pool).unwrap();
    let names: Vec<&str> = csv
        .lines()
        .skip(1)
        .map(|l| l.split(',').next().unwrap_or(""))
        .collect();
    assert_eq!(names, vec!["Alice", "bob", "charlie"]);
}

#[test]
fn export_escapes_fields_with_commas() {
    let pool = init_test_db();
    person_create(
        &pool,
        PersonCreateReq {
            display_name: "Smith, John".to_string(),
            email: None,
            role: None,
            note: Some("note with, comma".to_string()),
        },
    )
    .unwrap();

    let csv = export_persons_csv(&pool).unwrap();
    let data_line = csv.lines().nth(1).unwrap();
    // Fields with commas must be quoted
    assert!(data_line.starts_with('"'));
    assert!(data_line.contains("\"Smith, John\""));
    assert!(data_line.contains("\"note with, comma\""));
}

#[test]
fn export_escapes_fields_with_quotes() {
    let pool = init_test_db();
    person_create(
        &pool,
        PersonCreateReq {
            display_name: "O\"Brien".to_string(),
            email: None,
            role: None,
            note: None,
        },
    )
    .unwrap();

    let csv = export_persons_csv(&pool).unwrap();
    let data_line = csv.lines().nth(1).unwrap();
    // Double-quote escaping: " → ""
    assert!(data_line.contains("\"O\"\"Brien\""));
}

// ══════════════════════════════════════════════════════════
//  import_persons_csv
// ══════════════════════════════════════════════════════════

#[test]
fn import_creates_new_persons() {
    let pool = init_test_db();
    let csv = "display_name,email,role,note,is_active\n\
               Alice,alice@test.com,backend_developer,Senior dev,true\n\
               Bob,bob@test.com,tester,,true\n";

    let result = import_persons_csv(&pool, csv).unwrap();
    assert_eq!(result.created, 2);
    assert_eq!(result.updated, 0);
    assert_eq!(result.skipped, 0);
    assert!(result.errors.is_empty());

    // Verify persons are queryable
    let persons = app_lib::app::person_list(&pool, false).unwrap();
    assert_eq!(persons.len(), 2);
}

#[test]
fn import_updates_existing_person_by_name() {
    let pool = init_test_db();

    // Pre-create a person
    let created = person_create(
        &pool,
        PersonCreateReq {
            display_name: "Alice".to_string(),
            email: Some("old@test.com".to_string()),
            role: Some("tester".to_string()),
            note: None,
        },
    )
    .unwrap();

    // Import with updated fields
    let csv = "display_name,email,role,note,is_active\n\
               Alice,new@test.com,backend_developer,Updated note,true\n";
    let result = import_persons_csv(&pool, csv).unwrap();
    assert_eq!(result.created, 0);
    assert_eq!(result.updated, 1);
    assert_eq!(result.skipped, 0);

    // Verify fields were updated
    let updated = person_get(&pool, &created.id).unwrap();
    assert_eq!(updated.email, "new@test.com");
    assert_eq!(updated.role, "backend_developer");
    assert_eq!(updated.note, "Updated note");
}

#[test]
fn import_name_matching_is_case_insensitive() {
    let pool = init_test_db();
    person_create(
        &pool,
        PersonCreateReq {
            display_name: "Alice".to_string(),
            email: None,
            role: None,
            note: None,
        },
    )
    .unwrap();

    // Import with different casing
    let csv = "display_name,email,role,note,is_active\n\
               ALICE,alice@test.com,tester,,true\n";
    let result = import_persons_csv(&pool, csv).unwrap();
    // Should update, not create
    assert_eq!(result.created, 0);
    assert_eq!(result.updated, 1);

    // Only 1 person should exist
    let persons = app_lib::app::person_list(&pool, false).unwrap();
    assert_eq!(persons.len(), 1);
}

#[test]
fn import_idempotent_on_repeat() {
    let pool = init_test_db();
    let csv = "display_name,email,role,note,is_active\n\
               Alice,alice@test.com,tester,,true\n";

    let r1 = import_persons_csv(&pool, csv).unwrap();
    assert_eq!(r1.created, 1);
    assert_eq!(r1.updated, 0);

    // Re-import same data
    let r2 = import_persons_csv(&pool, csv).unwrap();
    assert_eq!(r2.created, 0);
    assert_eq!(r2.updated, 1);

    // Still only 1 person
    let persons = app_lib::app::person_list(&pool, false).unwrap();
    assert_eq!(persons.len(), 1);
}

#[test]
fn import_skips_empty_display_name_with_error() {
    let pool = init_test_db();
    let csv = "display_name,email,role,note,is_active\n\
               ,empty@test.com,tester,,true\n\
               Alice,alice@test.com,tester,,true\n";

    let result = import_persons_csv(&pool, csv).unwrap();
    assert_eq!(result.created, 1); // Alice created
    assert_eq!(result.skipped, 1); // empty name skipped
    assert_eq!(result.errors.len(), 1);
    assert!(result.errors[0].contains("Row 2"));
    assert!(result.errors[0].contains("display_name"));
}

#[test]
fn import_skips_row_with_too_few_columns() {
    let pool = init_test_db();
    let csv = "display_name,email,role,note,is_active\n\
               Alice,alice@test.com\n\
               Bob,bob@test.com,tester,,true\n";

    let result = import_persons_csv(&pool, csv).unwrap();
    assert_eq!(result.created, 1); // Bob created
    assert_eq!(result.skipped, 1); // Alice row skipped
    assert_eq!(result.errors.len(), 1);
    assert!(result.errors[0].contains("Row 2"));
    assert!(result.errors[0].contains("5 columns"));
}

#[test]
fn import_skips_invalid_is_active_value() {
    let pool = init_test_db();
    let csv = "display_name,email,role,note,is_active\n\
               Alice,alice@test.com,tester,,maybe\n\
               Bob,bob@test.com,tester,,true\n";

    let result = import_persons_csv(&pool, csv).unwrap();
    assert_eq!(result.created, 1); // Bob created
    assert_eq!(result.skipped, 1); // Alice skipped
    assert_eq!(result.errors.len(), 1);
    assert!(result.errors[0].contains("Row 2"));
    assert!(result.errors[0].contains("is_active"));
    assert!(result.errors[0].contains("maybe"));
}

#[test]
fn import_accepts_all_is_active_variants() {
    let pool = init_test_db();
    let csv = "display_name,email,role,note,is_active\n\
               Alice,,,,true\n\
               Bob,,,,false\n\
               Carol,,,,1\n\
               Dave,,,,0\n\
               Eve,,,,yes\n\
               Frank,,,,no\n";

    let result = import_persons_csv(&pool, csv).unwrap();
    assert_eq!(result.created, 6);
    assert_eq!(result.skipped, 0);
    assert!(result.errors.is_empty());

    let persons = app_lib::app::person_list(&pool, false).unwrap();
    let active: Vec<_> = persons.iter().filter(|p| p.is_active).collect();
    let inactive: Vec<_> = persons.iter().filter(|p| !p.is_active).collect();
    assert_eq!(active.len(), 3);   // Alice, Carol, Eve
    assert_eq!(inactive.len(), 3); // Bob, Dave, Frank
}

#[test]
fn import_invalid_header_returns_error() {
    let pool = init_test_db();
    let csv = "name,email,role,note,active\nAlice,,,, true\n";
    let result = import_persons_csv(&pool, csv);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), "VALIDATION_ERROR");
}

#[test]
fn import_handles_quoted_fields_with_commas() {
    let pool = init_test_db();
    let csv = "display_name,email,role,note,is_active\n\
               \"Smith, John\",john@test.com,tester,\"note with, comma\",true\n";

    let result = import_persons_csv(&pool, csv).unwrap();
    assert_eq!(result.created, 1);
    assert_eq!(result.skipped, 0);

    let persons = app_lib::app::person_list(&pool, false).unwrap();
    assert_eq!(persons[0].display_name, "Smith, John");
    assert_eq!(persons[0].note, "note with, comma");
}

#[test]
fn export_import_roundtrip_preserves_all_fields() {
    let pool1 = init_test_db();

    // Seed with varied data
    for (name, email, role, note) in &[
        ("Alice", "alice@test.com", "backend_developer", "Senior"),
        ("Bob", "", "tester", ""),
        ("Carol", "carol@test.com", "product_manager", "Lead"),
    ] {
        person_create(
            &pool1,
            PersonCreateReq {
                display_name: name.to_string(),
                email: if email.is_empty() { None } else { Some(email.to_string()) },
                role: if role.is_empty() { None } else { Some(role.to_string()) },
                note: if note.is_empty() { None } else { Some(note.to_string()) },
            },
        )
        .unwrap();
    }

    // Export
    let csv = export_persons_csv(&pool1).unwrap();

    // Import into fresh DB
    let pool2 = init_test_db();
    let result = import_persons_csv(&pool2, &csv).unwrap();
    assert_eq!(result.created, 3);
    assert_eq!(result.skipped, 0);

    // Verify all fields preserved
    let persons = app_lib::app::person_list(&pool2, false).unwrap();
    assert_eq!(persons.len(), 3);

    let alice = persons.iter().find(|p| p.display_name == "Alice").unwrap();
    assert_eq!(alice.email, "alice@test.com");
    assert_eq!(alice.role, "backend_developer");
    assert_eq!(alice.note, "Senior");
    assert!(alice.is_active);

    let bob = persons.iter().find(|p| p.display_name == "Bob").unwrap();
    assert_eq!(bob.email, "");
    assert_eq!(bob.role, "tester");
}

#[test]
fn import_skips_blank_lines_silently() {
    let pool = init_test_db();
    let csv = "display_name,email,role,note,is_active\n\
               \n\
               Alice,alice@test.com,tester,,true\n\
               \n";

    let result = import_persons_csv(&pool, csv).unwrap();
    assert_eq!(result.created, 1);
    assert_eq!(result.skipped, 0);
    assert!(result.errors.is_empty());
}
