//! S3 client wrapper for sync operations

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::config::Region;
use aws_sdk_s3::Client;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct S3ObjectSummary {
    pub key: String,
    pub last_modified_unix: Option<i64>,
}

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

        let force_path_style = should_force_path_style_for_endpoint(&endpoint);

        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(region_provider)
            .endpoint_url(endpoint)
            .credentials_provider(creds)
            .load()
            .await;

        // For local MinIO-style endpoints, use path-style to avoid bucket-subdomain parsing issues.
        // For cloud providers (AWS S3 / R2 / OSS), keep virtual-hosted style by default.
        let s3_config = aws_sdk_s3::config::Builder::from(&config)
            .force_path_style(force_path_style)
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

    /// List objects with prefix (paginated)
    pub async fn list(&self, prefix: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let summaries = self.list_with_metadata(prefix).await?;
        Ok(summaries.into_iter().map(|s| s.key).collect())
    }

    /// List objects with metadata (paginated).
    pub async fn list_with_metadata(
        &self,
        prefix: &str,
    ) -> Result<Vec<S3ObjectSummary>, Box<dyn std::error::Error>> {
        let mut continuation_token: Option<String> = None;
        let mut objects = Vec::new();

        loop {
            let mut req = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(prefix);

            if let Some(token) = &continuation_token {
                req = req.continuation_token(token);
            }

            let resp = req
                .send()
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

            for obj in resp.contents() {
                if let Some(key) = obj.key() {
                    objects.push(S3ObjectSummary {
                        key: key.to_string(),
                        last_modified_unix: obj.last_modified().map(|dt| dt.secs()),
                    });
                }
            }

            if resp.is_truncated().unwrap_or(false) {
                continuation_token = resp.next_continuation_token().map(ToString::to_string);
                if continuation_token.is_none() {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(objects)
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

fn should_force_path_style_for_endpoint(endpoint: &str) -> bool {
    let host = extract_endpoint_host(endpoint);
    let host_lc = host.to_ascii_lowercase();

    host_lc == "localhost"
        || host_lc == "127.0.0.1"
        || host_lc == "::1"
        || host_lc.ends_with(".nip.io")
        || host_lc.ends_with(".local")
        || host_lc.contains("minio")
}

fn extract_endpoint_host(endpoint: &str) -> String {
    let authority = endpoint
        .split("://")
        .nth(1)
        .unwrap_or(endpoint)
        .split('/')
        .next()
        .unwrap_or("")
        .trim();

    if authority.starts_with('[') {
        return authority
            .trim_start_matches('[')
            .split(']')
            .next()
            .unwrap_or("")
            .to_string();
    }

    authority.split(':').next().unwrap_or(authority).to_string()
}

#[cfg(test)]
mod tests {
    use super::should_force_path_style_for_endpoint;

    #[test]
    fn should_force_path_style_for_local_endpoints() {
        assert!(should_force_path_style_for_endpoint(
            "http://localhost:9000"
        ));
        assert!(should_force_path_style_for_endpoint(
            "http://127.0.0.1:9000"
        ));
        assert!(should_force_path_style_for_endpoint(
            "http://127.0.0.1.nip.io:9000"
        ));
        assert!(should_force_path_style_for_endpoint("http://minio:9000"));
    }

    #[test]
    fn should_not_force_path_style_for_cloud_endpoints() {
        assert!(!should_force_path_style_for_endpoint(
            "https://bucket.s3.us-east-1.amazonaws.com"
        ));
        assert!(!should_force_path_style_for_endpoint(
            "https://account-id.r2.cloudflarestorage.com"
        ));
        assert!(!should_force_path_style_for_endpoint(
            "https://oss-cn-shanghai.aliyuncs.com"
        ));
    }
}
