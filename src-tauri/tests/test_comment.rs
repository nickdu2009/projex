//! Comment CRUD integration tests

use app_lib::app::{
    comment_create, comment_delete, comment_list_by_project, comment_update, partner_create,
    person_create, project_create, CommentCreateReq, CommentUpdateReq, PartnerCreateReq,
    PersonCreateReq, ProjectCreateReq,
};
use app_lib::infra::db::init_test_db;

// ──────────────────────── Helper ────────────────────────

#[allow(dead_code)]
struct TestSeedIds {
    person_id: String,
    partner_id: String,
    project_id: String,
}

fn seed(pool: &app_lib::infra::DbPool) -> TestSeedIds {
    let person = person_create(
        pool,
        PersonCreateReq {
            display_name: "Test User".to_string(),
            email: Some("user@test.com".to_string()),
            role: Some("Developer".to_string()),
            note: None,
        },
    )
    .unwrap();

    let partner = partner_create(
        pool,
        PartnerCreateReq {
            name: format!("Partner-{}", uuid::Uuid::new_v4()),
            note: None,
        },
    )
    .unwrap();

    let project = project_create(
        pool,
        ProjectCreateReq {
            name: format!("Test Project-{}", uuid::Uuid::new_v4()),
            description: Some("Test description".to_string()),
            priority: Some(3),
            country_code: "CN".to_string(),
            partner_id: partner.id.clone(),
            owner_person_id: person.id.clone(),
            product_name: None,
            start_date: None,
            due_date: None,
            tags: None,
            created_by_person_id: Some(person.id.clone()),
        },
    )
    .unwrap();

    TestSeedIds {
        person_id: person.id,
        partner_id: partner.id,
        project_id: project.id,
    }
}

// ══════════════════════════════════════════════════════════
//  comment_create
// ══════════════════════════════════════════════════════════

#[test]
fn create_comment_without_person() {
    let pool = init_test_db();
    let ids = seed(&pool);

    let comment = comment_create(
        &pool,
        CommentCreateReq {
            project_id: ids.project_id.clone(),
            person_id: None,
            content: "{\"type\":\"doc\",\"content\":[]}".to_string(),
            is_pinned: None,
        },
    )
    .unwrap();

    assert_eq!(comment.project_id, ids.project_id);
    assert!(comment.person_id.is_none());
    assert!(comment.person_name.is_none());
    assert_eq!(comment.content, "{\"type\":\"doc\",\"content\":[]}");
    assert!(!comment.is_pinned);
    assert!(!comment.created_at.is_empty());
    assert!(!comment.updated_at.is_empty());
}

#[test]
fn create_comment_with_person() {
    let pool = init_test_db();
    let ids = seed(&pool);

    let comment = comment_create(
        &pool,
        CommentCreateReq {
            project_id: ids.project_id.clone(),
            person_id: Some(ids.person_id.clone()),
            content: "{\"type\":\"doc\"}".to_string(),
            is_pinned: Some(true),
        },
    )
    .unwrap();

    assert_eq!(comment.project_id, ids.project_id);
    assert_eq!(comment.person_id, Some(ids.person_id.clone()));
    assert_eq!(comment.person_name, Some("Test User".to_string()));
    assert!(comment.is_pinned);
}

#[test]
fn create_comment_project_not_found() {
    let pool = init_test_db();

    let result = comment_create(
        &pool,
        CommentCreateReq {
            project_id: "non-existent-project".to_string(),
            person_id: None,
            content: "{}".to_string(),
            is_pinned: None,
        },
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code(), "NOT_FOUND");
}

#[test]
fn create_comment_person_not_found() {
    let pool = init_test_db();
    let ids = seed(&pool);

    let result = comment_create(
        &pool,
        CommentCreateReq {
            project_id: ids.project_id.clone(),
            person_id: Some("non-existent-person".to_string()),
            content: "{}".to_string(),
            is_pinned: None,
        },
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code(), "NOT_FOUND");
}

// ══════════════════════════════════════════════════════════
//  comment_update
// ══════════════════════════════════════════════════════════

#[test]
fn update_comment_content() {
    let pool = init_test_db();
    let ids = seed(&pool);

    let comment = comment_create(
        &pool,
        CommentCreateReq {
            project_id: ids.project_id.clone(),
            person_id: None,
            content: "original".to_string(),
            is_pinned: None,
        },
    )
    .unwrap();

    let updated = comment_update(
        &pool,
        CommentUpdateReq {
            id: comment.id.clone(),
            content: Some("updated content".to_string()),
            person_id: None,
            is_pinned: None,
        },
    )
    .unwrap();

    assert_eq!(updated.id, comment.id);
    assert_eq!(updated.content, "updated content");
    assert_ne!(updated.updated_at, comment.updated_at);
}

#[test]
fn update_comment_toggle_pin() {
    let pool = init_test_db();
    let ids = seed(&pool);

    let comment = comment_create(
        &pool,
        CommentCreateReq {
            project_id: ids.project_id.clone(),
            person_id: None,
            content: "test".to_string(),
            is_pinned: Some(false),
        },
    )
    .unwrap();

    assert!(!comment.is_pinned);

    let updated = comment_update(
        &pool,
        CommentUpdateReq {
            id: comment.id.clone(),
            content: None,
            person_id: None,
            is_pinned: Some(true),
        },
    )
    .unwrap();

    assert!(updated.is_pinned);
    assert_eq!(updated.content, "test"); // content unchanged
}

#[test]
fn update_comment_assign_person() {
    let pool = init_test_db();
    let ids = seed(&pool);

    let comment = comment_create(
        &pool,
        CommentCreateReq {
            project_id: ids.project_id.clone(),
            person_id: None,
            content: "test".to_string(),
            is_pinned: None,
        },
    )
    .unwrap();

    assert!(comment.person_id.is_none());

    let updated = comment_update(
        &pool,
        CommentUpdateReq {
            id: comment.id.clone(),
            content: None,
            person_id: Some(ids.person_id.clone()),
            is_pinned: None,
        },
    )
    .unwrap();

    assert_eq!(updated.person_id, Some(ids.person_id));
    assert_eq!(updated.person_name, Some("Test User".to_string()));
}

#[test]
fn update_comment_not_found() {
    let pool = init_test_db();

    let result = comment_update(
        &pool,
        CommentUpdateReq {
            id: "non-existent".to_string(),
            content: Some("new".to_string()),
            person_id: None,
            is_pinned: None,
        },
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code(), "NOT_FOUND");
}

#[test]
fn update_comment_person_not_found() {
    let pool = init_test_db();
    let ids = seed(&pool);

    let comment = comment_create(
        &pool,
        CommentCreateReq {
            project_id: ids.project_id.clone(),
            person_id: None,
            content: "test".to_string(),
            is_pinned: None,
        },
    )
    .unwrap();

    let result = comment_update(
        &pool,
        CommentUpdateReq {
            id: comment.id.clone(),
            content: None,
            person_id: Some("non-existent-person".to_string()),
            is_pinned: None,
        },
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code(), "NOT_FOUND");
}

// ══════════════════════════════════════════════════════════
//  comment_delete
// ══════════════════════════════════════════════════════════

#[test]
fn delete_comment_succeeds() {
    let pool = init_test_db();
    let ids = seed(&pool);

    let comment = comment_create(
        &pool,
        CommentCreateReq {
            project_id: ids.project_id.clone(),
            person_id: None,
            content: "to be deleted".to_string(),
            is_pinned: None,
        },
    )
    .unwrap();

    let result = comment_delete(&pool, comment.id.clone());
    assert!(result.is_ok());

    // Verify it's gone
    let comments = comment_list_by_project(&pool, ids.project_id.clone()).unwrap();
    assert_eq!(comments.len(), 0);
}

#[test]
fn delete_comment_not_found() {
    let pool = init_test_db();

    let result = comment_delete(&pool, "non-existent".to_string());

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code(), "NOT_FOUND");
}

// ══════════════════════════════════════════════════════════
//  comment_list_by_project
// ══════════════════════════════════════════════════════════

#[test]
fn list_comments_empty() {
    let pool = init_test_db();
    let ids = seed(&pool);

    let comments = comment_list_by_project(&pool, ids.project_id.clone()).unwrap();
    assert_eq!(comments.len(), 0);
}

#[test]
fn list_comments_pinned_first() {
    let pool = init_test_db();
    let ids = seed(&pool);

    // Create 3 comments: unpinned, pinned, unpinned
    let c1 = comment_create(
        &pool,
        CommentCreateReq {
            project_id: ids.project_id.clone(),
            person_id: None,
            content: "comment 1".to_string(),
            is_pinned: Some(false),
        },
    )
    .unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10)); // Ensure different created_at

    let c2 = comment_create(
        &pool,
        CommentCreateReq {
            project_id: ids.project_id.clone(),
            person_id: None,
            content: "comment 2 (pinned)".to_string(),
            is_pinned: Some(true),
        },
    )
    .unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let c3 = comment_create(
        &pool,
        CommentCreateReq {
            project_id: ids.project_id.clone(),
            person_id: None,
            content: "comment 3".to_string(),
            is_pinned: Some(false),
        },
    )
    .unwrap();

    let comments = comment_list_by_project(&pool, ids.project_id.clone()).unwrap();

    assert_eq!(comments.len(), 3);
    // First should be pinned
    assert_eq!(comments[0].id, c2.id);
    assert!(comments[0].is_pinned);
    // Then unpinned, newest first
    assert_eq!(comments[1].id, c3.id);
    assert_eq!(comments[2].id, c1.id);
}

#[test]
fn list_comments_newest_first_when_not_pinned() {
    let pool = init_test_db();
    let ids = seed(&pool);

    let c1 = comment_create(
        &pool,
        CommentCreateReq {
            project_id: ids.project_id.clone(),
            person_id: None,
            content: "first".to_string(),
            is_pinned: None,
        },
    )
    .unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let c2 = comment_create(
        &pool,
        CommentCreateReq {
            project_id: ids.project_id.clone(),
            person_id: None,
            content: "second".to_string(),
            is_pinned: None,
        },
    )
    .unwrap();

    let comments = comment_list_by_project(&pool, ids.project_id.clone()).unwrap();

    assert_eq!(comments.len(), 2);
    assert_eq!(comments[0].id, c2.id); // Newest first
    assert_eq!(comments[1].id, c1.id);
}

#[test]
fn list_comments_filters_by_project() {
    let pool = init_test_db();
    let ids1 = seed(&pool);
    let ids2 = seed(&pool);

    comment_create(
        &pool,
        CommentCreateReq {
            project_id: ids1.project_id.clone(),
            person_id: None,
            content: "project 1 comment".to_string(),
            is_pinned: None,
        },
    )
    .unwrap();

    comment_create(
        &pool,
        CommentCreateReq {
            project_id: ids2.project_id.clone(),
            person_id: None,
            content: "project 2 comment".to_string(),
            is_pinned: None,
        },
    )
    .unwrap();

    let comments1 = comment_list_by_project(&pool, ids1.project_id.clone()).unwrap();
    let comments2 = comment_list_by_project(&pool, ids2.project_id.clone()).unwrap();

    assert_eq!(comments1.len(), 1);
    assert_eq!(comments2.len(), 1);
    assert_eq!(comments1[0].content, "project 1 comment");
    assert_eq!(comments2[0].content, "project 2 comment");
}
