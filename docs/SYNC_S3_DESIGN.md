# S3 同步方案技术设计

## 1. 方案概述

使用 AWS S3（或兼容服务）作为中心化存储，实现多设备数据同步。

### 核心优势

- ✅ **成本低**：$0-5/月（推荐 Cloudflare R2 零成本）
- ✅ **高可靠**：99.999999999% 数据持久性
- ✅ **简单**：无需维护服务器
- ✅ **跨平台**：所有设备都支持
- ✅ **全球 CDN**：访问速度快

### 支持的 S3 兼容服务

| 服务商 | 月费用 | 免费额度 | 推荐度 |
|--------|--------|----------|--------|
| **Cloudflare R2** | $0.015/GB | 10GB免费 + 零出站费用 | ⭐⭐⭐⭐⭐ |
| **AWS S3** | $0.023/GB | 12个月免费 | ⭐⭐⭐⭐ |
| **MinIO** | 自托管/$0 | 无限 | ⭐⭐⭐⭐ |
| **Backblaze B2** | $0.005/GB | 10GB免费 | ⭐⭐⭐⭐ |
| **阿里云 OSS** | ¥0.12/GB | 3个月免费 | ⭐⭐⭐⭐ |

**推荐：Cloudflare R2**（零出站费用，性价比最高）

---

## 2. 架构设计

### 2.1 整体架构

```mermaid
graph TB
    subgraph "设备 A (Mac)"
        A1[Tauri App A]
        A2[Local SQLite]
        A3[CRDT Engine]
    end
    
    subgraph "设备 B (Mac)"
        B1[Tauri App B]
        B2[Local SQLite]
        B3[CRDT Engine]
    end
    
    subgraph "S3 存储桶"
        S3_STATE["devices/<br/>设备状态"]
        S3_DELTAS["deltas/<br/>增量变更"]
        S3_SNAPSHOTS["snapshots/<br/>完整快照"]
    end
    
    A1 -->|读写| A2
    A2 -->|生成变更| A3
    A3 -->|上传| S3_DELTAS
    A3 -->|定期快照| S3_SNAPSHOTS
    A3 -->|注册| S3_STATE
    
    B3 -->|下载| S3_DELTAS
    B3 -->|拉取快照| S3_SNAPSHOTS
    B3 -->|读状态| S3_STATE
    B3 -->|应用变更| B2
    B2 -->|读写| B1
```

### 2.2 S3 目录结构

```
s3://my-project-sync/
├── devices/                    # 设备注册信息
│   ├── device-001.json        # 设备001信息
│   └── device-002.json        # 设备002信息
│
├── deltas/                     # 增量变更日志
│   ├── 2026/02/10/
│   │   ├── 001-device-001.json    # 变更序号-设备ID
│   │   ├── 002-device-002.json
│   │   └── 003-device-001.json
│   └── index.json             # 变更索引（版本号）
│
├── snapshots/                  # 完整数据快照
│   ├── snapshot-v1000.json.gz # 版本1000快照（压缩）
│   ├── snapshot-v2000.json.gz
│   └── latest.json            # 最新快照指针
│
└── metadata/                   # 元数据
    ├── schema-version.json    # 数据Schema版本
    └── sync-config.json       # 同步配置
```

### 2.3 数据流程

```
1. 本地变更 → 生成 Delta → 上传到 S3
2. 定时拉取 → 下载 Delta → 检测冲突 → 应用变更
3. 定期快照 → 压缩数据 → 上传完整备份
```

### 2.4 同步覆盖的数据表

以下表的 INSERT/UPDATE/DELETE 操作通过 SQLite 触发器自动记录到 `sync_metadata`，纳入 Delta 同步：

| 表名 | 触发器定义位置 | 说明 |
|------|---------------|------|
| `projects` | `0003_add_sync_support.sql` | 项目基础数据 |
| `persons` | `0003_add_sync_support.sql` | 成员数据 |
| `partners` | `0003_add_sync_support.sql` | 合作方数据 |
| `assignments` | `0003_add_sync_support.sql` | 成员参与记录 |
| `status_history` | `0003_add_sync_support.sql` | 状态变更历史 |
| `project_tags` | `0003_add_sync_support.sql` | 项目标签 |
| `project_comments` | `0004_add_project_comments.sql` | 项目评论（富文本）|

---

## 3. 核心实现

### 3.1 依赖配置

```toml
# src-tauri/Cargo.toml

[dependencies]
aws-sdk-s3 = "1.60"
aws-config = "1.5"
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = "0.4"
uuid = { version = "1.11", features = ["v4"] }
sha2 = "0.10"
flate2 = "1.0"  # 用于压缩
```

### 3.2 S3 客户端封装

```rust
// src-tauri/src/sync/s3_client.rs

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{Client, Error as S3Error};
use serde::{Deserialize, Serialize};

pub struct S3SyncClient {
    client: Client,
    bucket: String,
    device_id: String,
}

impl S3SyncClient {
    /// 创建标准 S3 客户端（AWS）
    pub async fn new(bucket: String, device_id: String) -> Result<Self, Box<dyn std::error::Error>> {
        let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
        let config = aws_config::from_env().region(region_provider).load().await;
        let client = Client::new(&config);
        
        Ok(Self {
            client,
            bucket,
            device_id,
        })
    }
    
    /// 创建自定义端点客户端（MinIO/R2/OSS等）
    pub async fn new_with_endpoint(
        bucket: String,
        device_id: String,
        endpoint: String,
        access_key: String,
        secret_key: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        use aws_credential_types::Credentials;
        
        let creds = Credentials::new(access_key, secret_key, None, None, "custom");
        
        let config = aws_config::from_env()
            .endpoint_url(endpoint)
            .credentials_provider(creds)
            .load()
            .await;
        
        let client = Client::new(&config);
        
        Ok(Self {
            client,
            bucket,
            device_id,
        })
    }
    
    /// 上传对象
    pub async fn upload(&self, key: &str, data: Vec<u8>) -> Result<(), S3Error> {
        let start = std::time::Instant::now();
        
        let result = self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(data.into())
            .send()
            .await;
        
        let elapsed = start.elapsed();
        
        match &result {
            Ok(_) => log::info!("S3 upload: {} ({:.2?})", key, elapsed),
            Err(e) => log::error!("S3 upload failed: {} - {:?}", key, e),
        }
        
        result?;
        Ok(())
    }
    
    /// 下载对象
    pub async fn download(&self, key: &str) -> Result<Vec<u8>, S3Error> {
        let resp = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;
        
        let data = resp.body.collect().await?.into_bytes().to_vec();
        Ok(data)
    }
    
    /// 列出对象（带前缀过滤）
    pub async fn list(&self, prefix: &str) -> Result<Vec<String>, S3Error> {
        let resp = self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix)
            .send()
            .await?;
        
        let keys = resp
            .contents()
            .iter()
            .filter_map(|obj| obj.key().map(String::from))
            .collect();
        
        Ok(keys)
    }
    
    /// 删除对象
    pub async fn delete(&self, key: &str) -> Result<(), S3Error> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;
        
        Ok(())
    }
    
    /// 检查对象是否存在
    pub async fn exists(&self, key: &str) -> Result<bool, S3Error> {
        match self.client
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
```

### 3.3 增量同步引擎

```rust
// src-tauri/src/sync/delta_sync.rs

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    pub id: String,
    pub device_id: String,
    pub version: i64,
    pub timestamp: String,
    pub operations: Vec<Operation>,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub table: String,
    pub record_id: String,
    pub op_type: OperationType,
    pub data: serde_json::Value,
    pub vector_clock: VectorClock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    Insert,
    Update,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorClock {
    pub clocks: HashMap<String, i64>,
}

impl VectorClock {
    pub fn new(device_id: String) -> Self {
        let mut clocks = HashMap::new();
        clocks.insert(device_id, 0);
        Self { clocks }
    }
    
    pub fn increment(&mut self, device_id: &str) {
        *self.clocks.entry(device_id.to_string()).or_insert(0) += 1;
    }
    
    pub fn merge(&mut self, other: &VectorClock) {
        for (device, clock) in &other.clocks {
            let entry = self.clocks.entry(device.clone()).or_insert(0);
            *entry = (*entry).max(*clock);
        }
    }
    
    /// 判断是否存在冲突
    pub fn conflicts_with(&self, other: &VectorClock) -> bool {
        let mut self_greater = false;
        let mut other_greater = false;
        
        let all_devices: std::collections::HashSet<_> = self
            .clocks
            .keys()
            .chain(other.clocks.keys())
            .collect();
        
        for device in all_devices {
            let self_clock = self.clocks.get(device).unwrap_or(&0);
            let other_clock = other.clocks.get(device).unwrap_or(&0);
            
            if self_clock > other_clock {
                self_greater = true;
            } else if other_clock > self_clock {
                other_greater = true;
            }
        }
        
        self_greater && other_greater
    }
}

pub struct DeltaSyncEngine {
    s3_client: crate::sync::s3_client::S3SyncClient,
    local_version: i64,
    vector_clock: VectorClock,
}

impl DeltaSyncEngine {
    pub fn new(s3_client: crate::sync::s3_client::S3SyncClient, device_id: String) -> Self {
        Self {
            s3_client,
            local_version: 0,
            vector_clock: VectorClock::new(device_id),
        }
    }
    
    /// 推送本地变更到 S3
    pub async fn push(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // 1. 获取本地未同步的变更
        let operations = self.get_local_changes().await?;
        
        if operations.is_empty() {
            return Ok(());
        }
        
        // 2. 递增版本和向量时钟
        self.local_version += 1;
        self.vector_clock.increment(&self.s3_client.device_id);
        
        // 3. 创建 Delta
        let delta = Delta {
            id: uuid::Uuid::new_v4().to_string(),
            device_id: self.s3_client.device_id.clone(),
            version: self.local_version,
            timestamp: Utc::now().to_rfc3339(),
            operations: operations.clone(),
            checksum: self.calculate_checksum(&operations),
        };
        
        // 4. 上传到 S3
        let key = format!(
            "deltas/{}/{:03}-{}.json",
            Utc::now().format("%Y/%m/%d"),
            delta.version,
            delta.device_id
        );
        
        let data = serde_json::to_vec(&delta)?;
        self.s3_client.upload(&key, data).await?;
        
        // 5. 更新索引
        self.update_index(&delta).await?;
        
        // 6. 标记本地已同步
        self.mark_synced(&operations).await?;
        
        log::info!("Pushed {} operations, version: {}", operations.len(), self.local_version);
        
        Ok(())
    }
    
    /// 从 S3 拉取远程变更
    pub async fn pull(&mut self) -> Result<Vec<Delta>, Box<dyn std::error::Error>> {
        // 1. 下载索引文件
        let index = self.download_index().await?;
        
        // 2. 找出新的 deltas（版本号大于本地，且非自己创建）
        let new_deltas: Vec<_> = index
            .into_iter()
            .filter(|d| d.version > self.local_version)
            .filter(|d| d.device_id != self.s3_client.device_id)
            .collect();
        
        if new_deltas.is_empty() {
            return Ok(vec![]);
        }
        
        // 3. 下载并应用 deltas
        let mut applied = Vec::new();
        
        for delta_meta in new_deltas {
            let key = format!(
                "deltas/{}/{:03}-{}.json",
                delta_meta.date,
                delta_meta.version,
                delta_meta.device_id
            );
            
            let data = self.s3_client.download(&key).await?;
            let delta: Delta = serde_json::from_slice(&data)?;
            
            // 验证校验和
            if delta.checksum != self.calculate_checksum(&delta.operations) {
                log::error!("Checksum mismatch for delta {}", delta.id);
                continue;
            }
            
            // 应用变更
            self.apply_delta(&delta).await?;
            applied.push(delta);
        }
        
        log::info!("Pulled and applied {} deltas", applied.len());
        
        Ok(applied)
    }
    
    /// 应用 Delta 到本地数据库
    async fn apply_delta(&mut self, delta: &Delta) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.get_connection();
        let tx = conn.transaction()?;
        
        for op in &delta.operations {
            // 检查冲突
            if let Some(local_clock) = self.get_local_vector_clock(&op.table, &op.record_id).await? {
                if local_clock.conflicts_with(&op.vector_clock) {
                    log::warn!("Conflict detected for {}/{}", op.table, op.record_id);
                    self.resolve_conflict(op, &local_clock).await?;
                    continue;
                }
            }
            
            // 应用操作
            match op.op_type {
                OperationType::Insert | OperationType::Update => {
                    // 使用 UPSERT 语法
                    let sql = format!(
                        "INSERT OR REPLACE INTO {} VALUES (...)",
                        op.table
                    );
                    // 执行 SQL (需要根据实际表结构实现)
                }
                OperationType::Delete => {
                    let sql = format!("DELETE FROM {} WHERE id = ?", op.table);
                    // 执行 SQL
                }
            }
            
            // 更新向量时钟
            self.vector_clock.merge(&op.vector_clock);
        }
        
        tx.commit()?;
        
        // 更新本地版本号
        self.local_version = self.local_version.max(delta.version);
        
        Ok(())
    }
    
    /// 冲突解决策略：Last Write Wins (LWW)
    async fn resolve_conflict(
        &self,
        op: &Operation,
        local_clock: &VectorClock,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 比较向量时钟的总和（更大的胜出）
        let remote_sum: i64 = op.vector_clock.clocks.values().sum();
        let local_sum: i64 = local_clock.clocks.values().sum();
        
        if remote_sum > local_sum {
            log::info!("Remote wins for {}/{}", op.table, op.record_id);
            // 应用远程变更
        } else {
            log::info!("Local wins for {}/{}", op.table, op.record_id);
            // 保留本地变更
        }
        
        Ok(())
    }
    
    /// 计算操作列表的 SHA256 校验和
    fn calculate_checksum(&self, operations: &[Operation]) -> String {
        use sha2::{Sha256, Digest};
        let data = serde_json::to_string(operations).unwrap();
        let hash = Sha256::digest(data.as_bytes());
        format!("{:x}", hash)
    }
    
    // 辅助方法（需要根据实际数据库实现）
    async fn get_local_changes(&self) -> Result<Vec<Operation>, Box<dyn std::error::Error>> {
        // 从 sync_metadata 表获取未同步的变更
        todo!("实现从本地数据库读取未同步变更")
    }
    
    async fn mark_synced(&self, operations: &[Operation]) -> Result<(), Box<dyn std::error::Error>> {
        // 标记这些操作已同步
        todo!("实现标记已同步")
    }
    
    async fn download_index(&self) -> Result<Vec<DeltaMeta>, Box<dyn std::error::Error>> {
        let data = self.s3_client.download("deltas/index.json").await?;
        Ok(serde_json::from_slice(&data)?)
    }
    
    async fn update_index(&self, delta: &Delta) -> Result<(), Box<dyn std::error::Error>> {
        // 下载现有索引，追加新 delta，重新上传
        todo!("实现索引更新")
    }
    
    fn get_connection(&self) -> rusqlite::Connection {
        todo!("获取数据库连接")
    }
    
    async fn get_local_vector_clock(
        &self,
        table: &str,
        record_id: &str,
    ) -> Result<Option<VectorClock>, Box<dyn std::error::Error>> {
        todo!("获取本地记录的向量时钟")
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct DeltaMeta {
    version: i64,
    device_id: String,
    date: String,
}
```

### 3.4 快照管理

```rust
// src-tauri/src/sync/snapshot.rs

use serde::{Deserialize, Serialize};
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use flate2::Compression;
use std::io::{Write, Read};

#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: i64,
    pub timestamp: String,
    pub tables: Vec<TableSnapshot>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TableSnapshot {
    pub name: String,
    pub records: Vec<serde_json::Value>,
}

pub struct SnapshotManager {
    s3_client: crate::sync::s3_client::S3SyncClient,
}

impl SnapshotManager {
    pub fn new(s3_client: crate::sync::s3_client::S3SyncClient) -> Self {
        Self { s3_client }
    }
    
    /// 创建并上传快照
    pub async fn create_snapshot(&self, version: i64) -> Result<(), Box<dyn std::error::Error>> {
        // 1. 导出所有表
        let snapshot = self.export_all_tables().await?;
        
        // 2. 压缩（使用 gzip）
        let compressed = self.compress(&snapshot)?;
        
        // 3. 上传到 S3
        let key = format!("snapshots/snapshot-v{}.json.gz", version);
        self.s3_client.upload(&key, compressed).await?;
        
        // 4. 更新最新快照指针
        self.update_latest_pointer(version).await?;
        
        log::info!("Created snapshot version {}", version);
        Ok(())
    }
    
    /// 从 S3 恢复快照
    pub async fn restore_snapshot(&self, version: Option<i64>) -> Result<(), Box<dyn std::error::Error>> {
        // 1. 确定快照版本（未指定则使用最新）
        let version = match version {
            Some(v) => v,
            None => self.get_latest_version().await?,
        };
        
        // 2. 下载快照
        let key = format!("snapshots/snapshot-v{}.json.gz", version);
        let compressed = self.s3_client.download(&key).await?;
        
        // 3. 解压
        let snapshot: Snapshot = self.decompress(&compressed)?;
        
        // 4. 清空本地数据库
        self.truncate_all_tables().await?;
        
        // 5. 导入数据
        for table in &snapshot.tables {
            self.import_table(table).await?;
        }
        
        log::info!("Restored snapshot version {}", version);
        Ok(())
    }
    
    /// 清理旧快照（保留最近 N 个）
    pub async fn cleanup_old_snapshots(&self, keep_count: usize) -> Result<(), Box<dyn std::error::Error>> {
        let snapshots = self.s3_client.list("snapshots/").await?;
        
        // 提取版本号并排序
        let mut versions: Vec<i64> = snapshots
            .iter()
            .filter_map(|key| {
                key.strip_prefix("snapshots/snapshot-v")
                    .and_then(|s| s.strip_suffix(".json.gz"))
                    .and_then(|s| s.parse().ok())
            })
            .collect();
        
        versions.sort_unstable();
        versions.reverse(); // 降序
        
        // 删除旧快照
        for version in versions.iter().skip(keep_count) {
            let key = format!("snapshots/snapshot-v{}.json.gz", version);
            self.s3_client.delete(&key).await?;
            log::info!("Deleted old snapshot version {}", version);
        }
        
        Ok(())
    }
    
    /// 压缩快照数据
    fn compress(&self, snapshot: &Snapshot) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let json = serde_json::to_vec(snapshot)?;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(&json)?;
        Ok(encoder.finish()?)
    }
    
    /// 解压快照数据
    fn decompress(&self, data: &[u8]) -> Result<Snapshot, Box<dyn std::error::Error>> {
        let mut decoder = GzDecoder::new(data);
        let mut json = Vec::new();
        decoder.read_to_end(&mut json)?;
        Ok(serde_json::from_slice(&json)?)
    }
    
    async fn export_all_tables(&self) -> Result<Snapshot, Box<dyn std::error::Error>> {
        todo!("实现数据库导出")
    }
    
    async fn get_latest_version(&self) -> Result<i64, Box<dyn std::error::Error>> {
        let data = self.s3_client.download("snapshots/latest.json").await?;
        let meta: serde_json::Value = serde_json::from_slice(&data)?;
        Ok(meta["version"].as_i64().unwrap())
    }
    
    async fn update_latest_pointer(&self, version: i64) -> Result<(), Box<dyn std::error::Error>> {
        let meta = serde_json::json!({
            "version": version,
            "updated_at": chrono::Utc::now().to_rfc3339(),
        });
        self.s3_client.upload("snapshots/latest.json", serde_json::to_vec(&meta)?).await?;
        Ok(())
    }
    
    async fn truncate_all_tables(&self) -> Result<(), Box<dyn std::error::Error>> {
        todo!("实现清空所有表")
    }
    
    async fn import_table(&self, table: &TableSnapshot) -> Result<(), Box<dyn std::error::Error>> {
        todo!("实现表数据导入")
    }
}
```

### 3.5 Tauri 命令

```rust
// src-tauri/src/commands/sync.rs

use tauri::State;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SyncState {
    pub engine: Arc<Mutex<Option<crate::sync::delta_sync::DeltaSyncEngine>>>,
}

#[tauri::command]
pub async fn sync_init(
    bucket: String,
    device_id: String,
    endpoint: Option<String>,
    access_key: Option<String>,
    secret_key: Option<String>,
    state: State<'_, SyncState>,
) -> Result<String, String> {
    use crate::sync::s3_client::S3SyncClient;
    use crate::sync::delta_sync::DeltaSyncEngine;
    
    let client = if let (Some(ep), Some(ak), Some(sk)) = (endpoint, access_key, secret_key) {
        S3SyncClient::new_with_endpoint(bucket, device_id.clone(), ep, ak, sk)
            .await
            .map_err(|e| e.to_string())?
    } else {
        S3SyncClient::new(bucket, device_id.clone())
            .await
            .map_err(|e| e.to_string())?
    };
    
    let engine = DeltaSyncEngine::new(client, device_id);
    
    let mut guard = state.engine.lock().await;
    *guard = Some(engine);
    
    Ok("Sync initialized".to_string())
}

#[tauri::command]
pub async fn sync_push(state: State<'_, SyncState>) -> Result<String, String> {
    let mut guard = state.engine.lock().await;
    
    if let Some(engine) = guard.as_mut() {
        engine.push().await.map_err(|e| e.to_string())?;
        Ok("Push completed".to_string())
    } else {
        Err("Sync not initialized".to_string())
    }
}

#[tauri::command]
pub async fn sync_pull(state: State<'_, SyncState>) -> Result<usize, String> {
    let mut guard = state.engine.lock().await;
    
    if let Some(engine) = guard.as_mut() {
        let deltas = engine.pull().await.map_err(|e| e.to_string())?;
        Ok(deltas.len())
    } else {
        Err("Sync not initialized".to_string())
    }
}

#[tauri::command]
pub async fn sync_full(state: State<'_, SyncState>) -> Result<String, String> {
    // 先拉取，后推送（避免覆盖）
    let pulled = sync_pull(state.clone()).await?;
    sync_push(state).await?;
    Ok(format!("Synced: {} changes pulled", pulled))
}

#[tauri::command]
pub async fn snapshot_create(
    version: i64,
    state: State<'_, SyncState>,
) -> Result<String, String> {
    // 创建快照
    todo!("实现快照创建命令")
}

#[tauri::command]
pub async fn snapshot_restore(
    version: Option<i64>,
    state: State<'_, SyncState>,
) -> Result<String, String> {
    // 恢复快照
    todo!("实现快照恢复命令")
}
```

### 3.6 同步与评论的自动集成

项目评论（`project_comments`）通过 `0004_add_project_comments.sql` 中定义的 INSERT/UPDATE/DELETE 触发器自动纳入同步流程。评论的 CRUD 操作（`cmd_comment_create`/`cmd_comment_update`/`cmd_comment_delete`）无需额外同步代码——触发器会自动将变更写入 `sync_metadata` 表，由 Delta Sync Engine 统一处理。

### 3.7 注册到 Tauri

```rust
// src-tauri/src/lib.rs

mod sync;

use sync::SyncState;
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(SyncState {
            engine: Arc::new(Mutex::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            // ... 现有命令 ...
            commands::sync::sync_init,
            commands::sync::sync_push,
            commands::sync::sync_pull,
            commands::sync::sync_full,
            commands::sync::snapshot_create,
            commands::sync::snapshot_restore,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## 4. 前端实现

### 4.1 S3 同步管理器

```typescript
// src/sync/s3-sync-manager.ts

import { invoke } from '@tauri-apps/api/core';

export interface SyncConfig {
  bucket: string;
  deviceId: string;
  endpoint?: string;
  accessKey?: string;
  secretKey?: string;
}

export interface SyncStatus {
  status: 'idle' | 'syncing' | 'error';
  lastSync: Date | null;
  error: string | null;
}

export class S3SyncManager {
  private syncInterval: number | null = null;
  private status: SyncStatus = {
    status: 'idle',
    lastSync: null,
    error: null,
  };
  private listeners: Array<(status: SyncStatus) => void> = [];
  
  constructor(private config: SyncConfig) {}
  
  /**
   * 初始化同步引擎
   */
  async initialize(): Promise<void> {
    try {
      await invoke('sync_init', this.config);
      console.log('S3 sync initialized');
    } catch (error) {
      console.error('Failed to initialize sync:', error);
      throw error;
    }
  }
  
  /**
   * 执行完整同步（拉取 + 推送）
   */
  async sync(): Promise<void> {
    if (this.status.status === 'syncing') {
      console.log('Sync already in progress');
      return;
    }
    
    this.updateStatus({ status: 'syncing', error: null });
    
    try {
      const result = await invoke<string>('sync_full');
      console.log('Sync completed:', result);
      
      this.updateStatus({
        status: 'idle',
        lastSync: new Date(),
        error: null,
      });
    } catch (error: any) {
      console.error('Sync failed:', error);
      
      this.updateStatus({
        status: 'error',
        error: error.message || 'Unknown error',
      });
      
      throw error;
    }
  }
  
  /**
   * 仅推送本地变更
   */
  async push(): Promise<void> {
    await invoke('sync_push');
  }
  
  /**
   * 仅拉取远程变更
   */
  async pull(): Promise<number> {
    return await invoke<number>('sync_pull');
  }
  
  /**
   * 创建快照
   */
  async createSnapshot(version: number): Promise<void> {
    await invoke('snapshot_create', { version });
  }
  
  /**
   * 恢复快照
   */
  async restoreSnapshot(version?: number): Promise<void> {
    await invoke('snapshot_restore', { version });
  }
  
  /**
   * 启动自动同步
   */
  startAutoSync(intervalMs: number = 60000): void {
    if (this.syncInterval !== null) {
      console.warn('Auto sync already started');
      return;
    }
    
    this.syncInterval = window.setInterval(() => {
      this.sync().catch(console.error);
    }, intervalMs);
    
    // 立即执行一次
    this.sync().catch(console.error);
    
    console.log(`Auto sync started (interval: ${intervalMs}ms)`);
  }
  
  /**
   * 停止自动同步
   */
  stopAutoSync(): void {
    if (this.syncInterval !== null) {
      clearInterval(this.syncInterval);
      this.syncInterval = null;
      console.log('Auto sync stopped');
    }
  }
  
  /**
   * 获取同步状态
   */
  getStatus(): SyncStatus {
    return { ...this.status };
  }
  
  /**
   * 监听状态变化
   */
  onStatusChange(callback: (status: SyncStatus) => void): () => void {
    this.listeners.push(callback);
    
    // 返回取消监听函数
    return () => {
      const index = this.listeners.indexOf(callback);
      if (index > -1) {
        this.listeners.splice(index, 1);
      }
    };
  }
  
  private updateStatus(partial: Partial<SyncStatus>): void {
    this.status = { ...this.status, ...partial };
    this.listeners.forEach(callback => callback(this.status));
  }
}
```

### 4.2 同步状态 UI 组件

```typescript
// src/components/SyncStatusBar.tsx

import { Badge, Button, Group, Loader, Text, Tooltip } from '@mantine/core';
import { IconCloud, IconCloudOff, IconRefresh } from '@tabler/icons-react';
import { useEffect, useState } from 'react';
import { formatDistanceToNow } from 'date-fns';
import { zhCN } from 'date-fns/locale';
import type { SyncStatus } from '../sync/s3-sync-manager';

interface SyncStatusBarProps {
  syncManager: any; // S3SyncManager
}

export function SyncStatusBar({ syncManager }: SyncStatusBarProps) {
  const [status, setStatus] = useState<SyncStatus>(syncManager.getStatus());
  const [syncing, setSyncing] = useState(false);
  
  useEffect(() => {
    // 监听状态变化
    const unsubscribe = syncManager.onStatusChange(setStatus);
    return unsubscribe;
  }, [syncManager]);
  
  const handleManualSync = async () => {
    setSyncing(true);
    try {
      await syncManager.sync();
    } finally {
      setSyncing(false);
    }
  };
  
  const getStatusColor = () => {
    switch (status.status) {
      case 'syncing': return 'blue';
      case 'error': return 'red';
      case 'idle': return status.lastSync ? 'green' : 'gray';
    }
  };
  
  const getStatusText = () => {
    if (status.status === 'syncing') return '同步中...';
    if (status.status === 'error') return `错误: ${status.error}`;
    if (status.lastSync) {
      return `上次同步: ${formatDistanceToNow(status.lastSync, { locale: zhCN, addSuffix: true })}`;
    }
    return '未同步';
  };
  
  return (
    <Group gap="xs" style={{ padding: '8px 16px', borderTop: '1px solid var(--mantine-color-gray-2)' }}>
      <Badge
        leftSection={
          status.status === 'syncing' ? (
            <Loader size="xs" />
          ) : status.status === 'error' ? (
            <IconCloudOff size={12} />
          ) : (
            <IconCloud size={12} />
          )
        }
        color={getStatusColor()}
        variant="light"
      >
        云同步
      </Badge>
      
      <Text size="xs" c="dimmed">
        {getStatusText()}
      </Text>
      
      <Tooltip label="立即同步">
        <Button
          variant="subtle"
          size="xs"
          onClick={handleManualSync}
          loading={syncing || status.status === 'syncing'}
          leftSection={<IconRefresh size={14} />}
        >
          同步
        </Button>
      </Tooltip>
    </Group>
  );
}
```

### 4.3 设置页面集成

```typescript
// src/pages/Settings.tsx (新增同步配置)

import { Switch, TextInput, PasswordInput } from '@mantine/core';
import { S3SyncManager } from '../sync/s3-sync-manager';

export function Settings() {
  const [syncConfig, setSyncConfig] = useState({
    enabled: false,
    bucket: '',
    endpoint: '',
    accessKey: '',
    secretKey: '',
  });
  
  const handleSaveSync = async () => {
    try {
      // 保存到本地配置
      await invoke('save_sync_config', { config: syncConfig });
      
      // 初始化同步
      const manager = new S3SyncManager({
        bucket: syncConfig.bucket,
        deviceId: await getDeviceId(),
        endpoint: syncConfig.endpoint,
        accessKey: syncConfig.accessKey,
        secretKey: syncConfig.secretKey,
      });
      
      await manager.initialize();
      
      // 启动自动同步
      if (syncConfig.enabled) {
        manager.startAutoSync();
      }
      
      showSuccess('同步配置已保存');
    } catch (error: any) {
      showError(error.message);
    }
  };
  
  return (
    <Stack gap="md">
      <Paper>
        <Stack gap="md">
          <Title order={4}>云同步设置</Title>
          
          <Switch
            label="启用云同步"
            checked={syncConfig.enabled}
            onChange={(e) => setSyncConfig({ ...syncConfig, enabled: e.currentTarget.checked })}
          />
          
          <TextInput
            label="S3 Bucket"
            placeholder="my-project-sync"
            value={syncConfig.bucket}
            onChange={(e) => setSyncConfig({ ...syncConfig, bucket: e.currentTarget.value })}
          />
          
          <TextInput
            label="Endpoint (可选)"
            placeholder="https://abc123.r2.cloudflarestorage.com"
            description="留空使用 AWS S3，或填写 Cloudflare R2/MinIO 等端点"
            value={syncConfig.endpoint}
            onChange={(e) => setSyncConfig({ ...syncConfig, endpoint: e.currentTarget.value })}
          />
          
          <TextInput
            label="Access Key"
            placeholder="AKIAIOSFODNN7EXAMPLE"
            value={syncConfig.accessKey}
            onChange={(e) => setSyncConfig({ ...syncConfig, accessKey: e.currentTarget.value })}
          />
          
          <PasswordInput
            label="Secret Key"
            placeholder="wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
            value={syncConfig.secretKey}
            onChange={(e) => setSyncConfig({ ...syncConfig, secretKey: e.currentTarget.value })}
          />
          
          <Group>
            <Button onClick={handleSaveSync}>保存配置</Button>
            <Button variant="light" onClick={() => testConnection()}>测试连接</Button>
          </Group>
        </Stack>
      </Paper>
    </Stack>
  );
}
```

---

## 5. 成本分析

### 5.1 Cloudflare R2（推荐 ⭐）

```
存储: 10 GB 免费，之后 $0.015/GB/月
读取: 免费（无限）✅
写入: 免费（每月100万次）✅
出站流量: 免费 ✅

月费用估算（个人使用）:
- 数据库快照 (50MB) × 4周 = 200 MB
- 增量日志 (1MB/天) × 30天 = 30 MB
- 总计: ~250 MB → 免费额度内

年费用: $0 🎉
```

### 5.2 AWS S3

```
存储: $0.023/GB/月
读取: $0.0004/1000次
写入: $0.005/1000次
出站流量: $0.09/GB（前 100GB/月）

月费用估算:
- 存储: 0.25 GB × $0.023 = $0.006
- 写入: 100次/天 × 30 × $0.005/1000 = $0.015
- 读取: 100次/天 × 30 × $0.0004/1000 = $0.001
- 流量: 0.01 GB × $0.09 = $0.001
总计: ~$0.02/月

年费用: ~$0.25
```

### 5.3 MinIO（自托管）

```
服务器: DigitalOcean Droplet $6/月
存储: 包含 50GB SSD
带宽: 2TB/月

月费用: $6
年费用: $72

优点: 完全可控，数据私有
缺点: 需要维护
```

**结论：推荐 Cloudflare R2（零成本）**

---

## 6. 安全性

### 6.1 客户端加密（端到端）

```rust
// src-tauri/src/sync/encryption.rs

use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, NewAead};
use argon2::{self, Config};

pub struct E2EEncryption {
    cipher: Aes256Gcm,
}

impl E2EEncryption {
    /// 从用户密码派生加密密钥
    pub fn from_password(password: &str, salt: &[u8]) -> Self {
        let config = Config::default();
        let key_bytes = argon2::hash_raw(password.as_bytes(), salt, &config)
            .expect("Key derivation failed");
        
        let key = Key::from_slice(&key_bytes[0..32]);
        let cipher = Aes256Gcm::new(key);
        
        Self { cipher }
    }
    
    /// 加密数据
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let nonce = Self::generate_nonce();
        let ciphertext = self.cipher
            .encrypt(Nonce::from_slice(&nonce), plaintext)
            .map_err(|e| e.to_string())?;
        
        // 格式: [nonce (12 bytes)][ciphertext]
        let mut result = nonce.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }
    
    /// 解密数据
    pub fn decrypt(&self, encrypted: &[u8]) -> Result<Vec<u8>, String> {
        if encrypted.len() < 12 {
            return Err("Invalid encrypted data".to_string());
        }
        
        let (nonce, ciphertext) = encrypted.split_at(12);
        
        self.cipher
            .decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(|e| e.to_string())
    }
    
    fn generate_nonce() -> [u8; 12] {
        use rand::RngCore;
        let mut nonce = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce);
        nonce
    }
}
```

### 6.2 S3 Bucket 策略（IAM 权限）

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "AllowUserAccess",
      "Effect": "Allow",
      "Principal": {
        "AWS": "arn:aws:iam::ACCOUNT_ID:user/project-sync-user"
      },
      "Action": [
        "s3:GetObject",
        "s3:PutObject",
        "s3:DeleteObject",
        "s3:ListBucket"
      ],
      "Resource": [
        "arn:aws:s3:::my-project-sync/*",
        "arn:aws:s3:::my-project-sync"
      ]
    }
  ]
}
```

---

## 7. 部署步骤

### 7.1 Cloudflare R2 设置

```bash
# 1. 创建 R2 Bucket
# 访问 Cloudflare Dashboard → R2 → Create Bucket
# Bucket 名称: project-management-sync

# 2. 创建 API Token
# R2 → Manage R2 API Tokens → Create API Token
# 权限: Object Read & Write
# 保存: Access Key ID 和 Secret Access Key

# 3. 获取 Endpoint URL
# 格式: https://<account-id>.r2.cloudflarestorage.com
```

### 7.2 应用配置

```toml
# src-tauri/sync-config.toml

[s3]
provider = "cloudflare-r2"
bucket = "project-management-sync"
endpoint = "https://abc123.r2.cloudflarestorage.com"

[sync]
auto_sync_interval_seconds = 60
snapshot_interval_hours = 24
keep_snapshots_count = 7

[encryption]
enabled = true
# 密码由用户在首次启动时设置
```

### 7.3 环境变量

```bash
# .env (开发环境)
AWS_ACCESS_KEY_ID=your_access_key
AWS_SECRET_ACCESS_KEY=your_secret_key
AWS_REGION=auto  # R2 使用 "auto"
```

---

## 8. 测试

### 8.1 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_s3_upload_download() {
        let client = S3SyncClient::new(...).await.unwrap();
        
        let data = b"test data";
        client.upload("test.txt", data.to_vec()).await.unwrap();
        
        let downloaded = client.download("test.txt").await.unwrap();
        assert_eq!(data, downloaded.as_slice());
        
        client.delete("test.txt").await.unwrap();
    }
    
    #[test]
    fn test_vector_clock_merge() {
        let mut clock1 = VectorClock::new("device-1".to_string());
        let mut clock2 = VectorClock::new("device-2".to_string());
        
        clock1.increment("device-1");
        clock2.increment("device-2");
        
        clock1.merge(&clock2);
        
        assert_eq!(clock1.clocks.get("device-1"), Some(&1));
        assert_eq!(clock1.clocks.get("device-2"), Some(&1));
    }
}
```

### 8.2 集成测试

```typescript
// tests/sync.test.ts

describe('S3 Sync', () => {
  test('should sync between two devices', async () => {
    // 设备 A 创建项目
    const projectA = await createProject({ name: 'Test Project' });
    
    // 设备 A 推送
    await syncManagerA.push();
    
    // 设备 B 拉取
    await syncManagerB.pull();
    
    // 验证设备 B 有该项目
    const projectB = await getProject(projectA.id);
    expect(projectB.name).toBe('Test Project');
  });
  
  test('should resolve conflicts with LWW', async () => {
    // 两个设备同时修改
    await updateProject(projectId, { name: 'Name A' }); // 设备 A
    await updateProject(projectId, { name: 'Name B' }); // 设备 B
    
    // 同步
    await syncManagerA.sync();
    await syncManagerB.sync();
    
    // 验证最终一致
    const finalProject = await getProject(projectId);
    expect(['Name A', 'Name B']).toContain(finalProject.name);
  });
});
```

---

## 9. 实施计划

### 时间线（3-4周）

```
Week 1: 基础设施
  - Day 1-2: 添加依赖，S3 客户端封装
  - Day 3-4: Delta 数据结构设计
  - Day 5: 单元测试

Week 2: 同步引擎
  - Day 1-2: 推送逻辑实现
  - Day 3-4: 拉取逻辑实现
  - Day 5: 向量时钟和冲突解决

Week 3: 快照 + 前端
  - Day 1-2: 快照管理器
  - Day 3-4: 前端同步管理器
  - Day 5: UI 组件集成

Week 4: 测试 + 优化
  - Day 1-2: 集成测试
  - Day 3-4: 性能优化
  - Day 5: 文档完善
```

### 里程碑检查点

- ✅ **M1**: S3 上传/下载功能验证
- ✅ **M2**: 单设备同步工作正常
- ✅ **M3**: 多设备同步测试通过
- ✅ **M4**: 冲突场景测试通过
- ✅ **M5**: 性能达标，文档齐全

---

## 10. 监控与运维

### 10.1 同步指标

```typescript
export interface SyncMetrics {
  totalSyncs: number;
  successfulSyncs: number;
  failedSyncs: number;
  averageLatency: number;
  lastError: string | null;
}
```

### 10.2 日志记录

```rust
// 记录每次同步操作
log::info!("Sync started: device={}", device_id);
log::info!("Pushed {} deltas", delta_count);
log::info!("Pulled {} deltas", delta_count);
log::error!("Sync failed: {}", error);
```

---

## 11. 总结

### 核心优势

- ✅ **零成本**：使用 Cloudflare R2 完全免费
- ✅ **高可靠**：99.999999999% 数据持久性
- ✅ **易维护**：无需服务器，自动扩展
- ✅ **安全**：支持端到端加密
- ✅ **灵活**：支持所有 S3 兼容服务

### 适用场景

- ✅ 个人或小团队（< 10人）
- ✅ 预算有限
- ✅ 不需要实时协同（分钟级延迟可接受）
- ✅ 不想维护服务器

### 下一步行动

1. **配置 R2**（10分钟）
2. **添加依赖**（5分钟）
3. **复制核心代码**（1小时）
4. **测试验证**（2小时）
5. **集成到应用**（1天）

**预计总工作量：3-4周**

---

## 参考资料

- [AWS SDK for Rust](https://github.com/awslabs/aws-sdk-rust)
- [Cloudflare R2 文档](https://developers.cloudflare.com/r2/)
- [Vector Clock 论文](https://en.wikipedia.org/wiki/Vector_clock)
- [CRDT 简介](https://crdt.tech/)
