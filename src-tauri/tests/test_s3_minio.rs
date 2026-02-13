//! Optional MinIO integration tests for S3 client.
//! Run only when SYNC_MINIO_TEST=1 is set.

use app_lib::sync::S3SyncClient;
use std::env;
use uuid::Uuid;

#[derive(Debug, Clone)]
struct MinioTestConfig {
    endpoint: String,
    bucket: String,
    access_key: String,
    secret_key: String,
    device_id: String,
    object_prefix: String,
}

impl MinioTestConfig {
    fn from_env() -> Option<Self> {
        if env::var("SYNC_MINIO_TEST").ok().as_deref() != Some("1") {
            return None;
        }

        Some(Self {
            endpoint: required_with_fallback("SYNC_TEST_S3_ENDPOINT", "SYNC_S3_ENDPOINT"),
            bucket: required_with_fallback("SYNC_TEST_S3_BUCKET", "SYNC_S3_BUCKET"),
            access_key: required_with_fallback("SYNC_TEST_S3_ACCESS_KEY", "SYNC_S3_ACCESS_KEY"),
            secret_key: required_with_fallback("SYNC_TEST_S3_SECRET_KEY", "SYNC_S3_SECRET_KEY"),
            device_id: env::var("SYNC_TEST_S3_DEVICE_ID")
                .unwrap_or_else(|_| "minio-test-device".to_string()),
            object_prefix: env::var("SYNC_TEST_OBJECT_PREFIX")
                .unwrap_or_else(|_| "tests/minio".to_string()),
        })
    }
}

fn required_with_fallback(primary: &str, fallback: &str) -> String {
    env::var(primary)
        .or_else(|_| env::var(fallback))
        .unwrap_or_else(|_| panic!("Missing env var: {} (or fallback {})", primary, fallback))
}

fn make_client(cfg: &MinioTestConfig) -> S3SyncClient {
    let rt = tokio::runtime::Runtime::new().expect("create tokio runtime");
    rt.block_on(async {
        S3SyncClient::new_with_endpoint(
            cfg.bucket.clone(),
            cfg.device_id.clone(),
            cfg.endpoint.clone(),
            cfg.access_key.clone(),
            cfg.secret_key.clone(),
        )
        .await
        .expect("create S3 client with custom endpoint")
    })
}

#[test]
fn minio_smoke_upload_download_delete() {
    let Some(cfg) = MinioTestConfig::from_env() else {
        eprintln!("skip minio_smoke_upload_download_delete: SYNC_MINIO_TEST != 1");
        return;
    };

    let client = make_client(&cfg);
    let key = format!(
        "{}/smoke-{}/hello.txt",
        cfg.object_prefix,
        Uuid::new_v4()
            .to_string()
            .replace('-', "")
            .chars()
            .take(8)
            .collect::<String>()
    );
    let payload = b"projex-minio-smoke".to_vec();

    let rt = tokio::runtime::Runtime::new().expect("create tokio runtime");
    rt.block_on(async {
        client
            .upload(&key, payload.clone())
            .await
            .expect("upload smoke object");

        let exists = client.exists(&key).await.expect("head smoke object");
        assert!(exists, "uploaded object should exist");

        let downloaded = client.download(&key).await.expect("download smoke object");
        assert_eq!(downloaded, payload, "downloaded payload should match");

        let listed = client
            .list(&format!("{}/", cfg.object_prefix))
            .await
            .expect("list smoke objects");
        assert!(
            listed.iter().any(|k| k == &key),
            "list result should contain uploaded key"
        );

        client.delete(&key).await.expect("delete smoke object");
        let exists_after_delete = client.exists(&key).await.expect("head after delete");
        assert!(!exists_after_delete, "deleted object should not exist");
    });
}

#[test]
fn minio_list_pagination_returns_all_objects() {
    let Some(cfg) = MinioTestConfig::from_env() else {
        eprintln!("skip minio_list_pagination_returns_all_objects: SYNC_MINIO_TEST != 1");
        return;
    };

    let client = make_client(&cfg);
    let run_id = Uuid::new_v4()
        .to_string()
        .replace('-', "")
        .chars()
        .take(10)
        .collect::<String>();
    let prefix = format!("{}/pagination-{}/", cfg.object_prefix, run_id);

    // >1000 is important to verify list_objects_v2 pagination.
    let object_count = 1005usize;
    let mut keys = Vec::with_capacity(object_count);
    for i in 0..object_count {
        keys.push(format!("{}obj-{:04}.txt", prefix, i));
    }

    let rt = tokio::runtime::Runtime::new().expect("create tokio runtime");
    rt.block_on(async {
        for key in &keys {
            client
                .upload(key, b"x".to_vec())
                .await
                .expect("upload pagination object");
        }

        let listed = client.list(&prefix).await.expect("list paginated keys");
        assert_eq!(
            listed.len(),
            object_count,
            "list() should return all keys across pages"
        );

        let listed_with_meta = client
            .list_with_metadata(&prefix)
            .await
            .expect("list paginated keys with metadata");
        assert_eq!(
            listed_with_meta.len(),
            object_count,
            "list_with_metadata() should return all keys across pages"
        );

        for key in &keys {
            assert!(
                listed.iter().any(|v| v == key),
                "missing key from paginated list: {}",
                key
            );
        }

        // Best-effort cleanup.
        for key in &keys {
            let _ = client.delete(key).await;
        }
    });
}
