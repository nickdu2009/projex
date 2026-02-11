//! S3 client wrapper for sync operations

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client;
use std::time::Instant;
use aws_sdk_s3::config::Region;

pub struct S3SyncClient {
    client: Client,
    pub bucket: String,
    pub device_id: String,
}

impl S3SyncClient {
    /// Create client with AWS credentials from environment
    pub async fn new(
        bucket: String,
        device_id: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;
        let client = Client::new(&config);

        Ok(Self {
            client,
            bucket,
            device_id,
        })
    }

    /// Create client with custom endpoint (for MinIO, R2, etc.)
    pub async fn new_with_endpoint(
        bucket: String,
        device_id: String,
        endpoint: String,
        access_key: String,
        secret_key: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        use aws_credential_types::Credentials;

        let creds = Credentials::new(access_key, secret_key, None, None, "custom");

        let region_provider = if let Some(region) = infer_region_from_endpoint(&endpoint) {
            RegionProviderChain::first_try(Region::new(region)).or_else("us-east-1")
        } else {
            RegionProviderChain::default_provider().or_else("us-east-1")
        };

        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(region_provider)
            .endpoint_url(endpoint)
            .credentials_provider(creds)
            .load()
            .await;

        // Use virtual-hosted style for S3-compatible endpoints.
        // For Aliyun OSS, this is required (SecondLevelDomainForbidden).
        let s3_config = aws_sdk_s3::config::Builder::from(&config)
            .force_path_style(false)
            .build();
        let client = Client::from_conf(s3_config);

        Ok(Self {
            client,
            bucket,
            device_id,
        })
    }

    /// Upload object to S3
    pub async fn upload(&self, key: &str, data: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        let start = Instant::now();
        let data_len = data.len();

        let result = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(data.into())
            .send()
            .await;

        let elapsed = start.elapsed();

        match &result {
            Ok(_) => log::info!("S3 upload: {} ({:.2?}, {} bytes)", key, elapsed, data_len),
            Err(e) => log::error!("S3 upload failed: {} - {:?}", key, e),
        }

        result.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        Ok(())
    }

    /// Download object from S3
    pub async fn download(&self, key: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let start = Instant::now();

        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        let data = resp
            .body
            .collect()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?
            .into_bytes()
            .to_vec();
        let elapsed = start.elapsed();

        log::info!(
            "S3 download: {} ({:.2?}, {} bytes)",
            key,
            elapsed,
            data.len()
        );

        Ok(data)
    }

    /// List objects with prefix
    pub async fn list(&self, prefix: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let resp = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix)
            .send()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        let keys = resp
            .contents()
            .iter()
            .filter_map(|obj| obj.key().map(String::from))
            .collect();

        Ok(keys)
    }

    /// Test connection to bucket with minimal request.
    pub async fn test_connection(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .max_keys(1)
            .send()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        Ok(())
    }

    /// Delete object from S3
    pub async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        log::info!("S3 deleted: {}", key);

        Ok(())
    }

    /// Check if object exists
    pub async fn exists(&self, key: &str) -> Result<bool, Box<dyn std::error::Error>> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

fn infer_region_from_endpoint(endpoint: &str) -> Option<String> {
    // Heuristics for common S3-compatible endpoints.
    // - Aliyun OSS: "oss-cn-shanghai.aliyuncs.com" -> "oss-cn-shanghai"
    // - Cloudflare R2: region is typically "auto"
    let host = endpoint
        .split("://")
        .nth(1)
        .unwrap_or(endpoint)
        .split('/')
        .next()
        .unwrap_or("");

    if host.contains("r2.cloudflarestorage.com") {
        return Some("auto".to_string());
    }

    // Aliyun OSS patterns:
    // - "oss-cn-shanghai.aliyuncs.com" -> "oss-cn-shanghai"
    // - "s3.oss-cn-shanghai.aliyuncs.com" -> "oss-cn-shanghai"
    for label in host.split('.') {
        if label.starts_with("oss-") {
            return Some(label.to_string());
        }
    }

    None
}

