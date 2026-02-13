//! Tauri commands for log viewing.

use crate::error::AppError;
use crate::infra::DbPool;
use crate::AppRuntimeState;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use tauri::State;

// 最大读取字节数上限：2MB
const MAX_TAIL_BYTES: usize = 2 * 1024 * 1024;
// 默认读取字节数：256KB
const DEFAULT_TAIL_BYTES: usize = 256 * 1024;

/// Log file metadata DTO
#[derive(Debug, Serialize)]
pub struct LogFileDto {
    pub name: String,
    pub size_bytes: u64,
    pub modified_at: Option<String>,
}

/// Log tail request DTO
#[derive(Debug, Deserialize)]
pub struct LogTailReq {
    pub file_name: String,
    #[serde(default = "default_max_bytes")]
    pub max_bytes: usize,
    #[serde(default = "default_redact")]
    pub redact: bool,
    /// Optional cursor for pagination (byte offset from end of file).
    pub cursor: Option<u64>,
}

fn default_max_bytes() -> usize {
    DEFAULT_TAIL_BYTES
}

fn default_redact() -> bool {
    true
}

/// Log tail response DTO
#[derive(Debug, Serialize)]
pub struct LogTailResp {
    pub content: String,
    /// Next cursor for pagination (byte offset from end).
    /// None means no more data to load.
    pub next_cursor: Option<u64>,
    pub truncated: bool,
}

/// Log clear request DTO
#[derive(Debug, Deserialize)]
pub struct LogClearReq {
    pub file_name: String,
}

fn allowed_log_bases(profile_name: &str) -> [String; 2] {
    [
        format!("rust-{}.log", profile_name),
        format!("webview-{}.log", profile_name),
    ]
}

/// 白名单文件名校验
/// 仅允许当前 profile 对应的 rust/webview 日志及轮转后缀（例如 xxx.log.1, xxx.log.2）
fn validate_log_file_name(name: &str, profile_name: &str) -> Result<(), AppError> {
    let allowed_bases = allowed_log_bases(profile_name);

    // 直接匹配基础文件名
    if allowed_bases.iter().any(|base| base == name) {
        return Ok(());
    }

    // 检查轮转后缀：base.N (N 是数字)
    for base in &allowed_bases {
        if let Some(suffix) = name.strip_prefix(base) {
            if let Some(num_part) = suffix.strip_prefix('.') {
                if !num_part.is_empty() && num_part.chars().all(|c| c.is_ascii_digit()) {
                    return Ok(());
                }
            }
        }
    }

    Err(AppError::LogFile(format!(
        "Invalid log file name: {}. Allowed files for profile '{}': {}, {} and rotated versions.",
        name, profile_name, allowed_bases[0], allowed_bases[1]
    )))
}

/// 获取日志目录路径
fn get_log_dir(runtime: &AppRuntimeState) -> Result<PathBuf, AppError> {
    let log_dir = runtime.log_dir();
    fs::create_dir_all(&log_dir)
        .map_err(|e| AppError::LogIo(format!("Failed to ensure log dir {:?}: {}", log_dir, e)))?;
    Ok(log_dir)
}

/// 从 sync_config 读取需要脱敏的凭据
fn get_redaction_patterns(conn: &Connection) -> Vec<String> {
    let mut patterns = Vec::new();

    // S3 access key
    if let Ok(key) = get_config_value(conn, "s3_access_key") {
        let trimmed = key.trim();
        if !trimmed.is_empty() {
            patterns.push(trimmed.to_string());
        }
    }

    // S3 secret key
    if let Ok(key) = get_config_value(conn, "s3_secret_key") {
        let trimmed = key.trim();
        if !trimmed.is_empty() {
            patterns.push(trimmed.to_string());
        }
    }

    patterns
}

fn get_config_value(conn: &Connection, key: &str) -> Result<String, rusqlite::Error> {
    conn.query_row(
        "SELECT value FROM sync_config WHERE key = ?1",
        [key],
        |row| row.get(0),
    )
}

/// 脱敏处理：替换敏感信息为 ***
fn redact_content(content: &str, patterns: &[String]) -> String {
    let mut result = content.to_string();
    for pattern in patterns {
        if pattern.len() >= 4 {
            result = result.replace(pattern, "***");
        }
    }
    result
}

/// List all log files in the app log directory.
#[tauri::command]
pub fn cmd_log_list_files(
    runtime: State<'_, AppRuntimeState>,
) -> Result<Vec<LogFileDto>, AppError> {
    let log_dir = get_log_dir(runtime.inner())?;
    let profile_name = runtime.profile_name();

    if !log_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();

    for entry in fs::read_dir(&log_dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // 只返回白名单内的文件
        if validate_log_file_name(&file_name, profile_name).is_err() {
            continue;
        }

        let metadata = entry.metadata()?;
        let size_bytes = metadata.len();
        let modified_at = metadata
            .modified()
            .ok()
            .and_then(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .ok()
                    .map(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
            })
            .flatten()
            .map(|dt| dt.to_rfc3339());

        files.push(LogFileDto {
            name: file_name,
            size_bytes,
            modified_at,
        });
    }

    // 按修改时间倒序排序（最新的在前）
    files.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));

    Ok(files)
}

/// Read the tail of a log file with optional redaction.
#[tauri::command]
pub fn cmd_log_tail(
    pool: State<DbPool>,
    runtime: State<'_, AppRuntimeState>,
    req: LogTailReq,
) -> Result<LogTailResp, AppError> {
    // 白名单校验
    validate_log_file_name(&req.file_name, runtime.profile_name())?;

    // 限制 max_bytes
    let max_bytes = req.max_bytes.min(MAX_TAIL_BYTES);

    let log_dir = get_log_dir(runtime.inner())?;
    let file_path = log_dir.join(&req.file_name);

    if !file_path.exists() {
        return Err(AppError::NotFound(format!(
            "Log file not found: {}",
            req.file_name
        )));
    }

    let mut file = File::open(&file_path)?;
    let file_size = file.metadata()?.len();

    // 计算读取起始位置
    let current_cursor = req.cursor.unwrap_or(0);
    let bytes_to_read = max_bytes as u64;
    let start_offset = if current_cursor + bytes_to_read >= file_size {
        0
    } else {
        file_size - current_cursor - bytes_to_read
    };

    let actual_bytes_to_read = (file_size - start_offset).min(bytes_to_read);

    // 读取内容
    file.seek(SeekFrom::Start(start_offset))?;
    let mut buffer = vec![0u8; actual_bytes_to_read as usize];
    file.read_exact(&mut buffer)?;

    let mut content = String::from_utf8_lossy(&buffer).to_string();

    // 脱敏处理
    if req.redact {
        let conn = pool
            .inner()
            .0
            .lock()
            .map_err(|e| AppError::Db(e.to_string()))?;
        let patterns = get_redaction_patterns(&conn);
        if !patterns.is_empty() {
            content = redact_content(&content, &patterns);
        }
    }

    // 计算下一个游标
    let next_cursor = if start_offset > 0 {
        Some(current_cursor + actual_bytes_to_read)
    } else {
        None
    };

    let truncated = actual_bytes_to_read < bytes_to_read;

    Ok(LogTailResp {
        content,
        next_cursor,
        truncated,
    })
}

/// Clear (truncate) a log file.
#[tauri::command]
pub fn cmd_log_clear(
    runtime: State<'_, AppRuntimeState>,
    req: LogClearReq,
) -> Result<String, AppError> {
    // 白名单校验
    validate_log_file_name(&req.file_name, runtime.profile_name())?;

    let log_dir = get_log_dir(runtime.inner())?;
    let file_path = log_dir.join(&req.file_name);

    if !file_path.exists() {
        return Err(AppError::NotFound(format!(
            "Log file not found: {}",
            req.file_name
        )));
    }

    // 截断文件（清空内容但保留文件）
    fs::write(&file_path, "")?;

    Ok(format!("Log file {} cleared successfully", req.file_name))
}

/// Log level DTO
#[derive(Debug, Serialize)]
pub struct LogLevelResp {
    pub current_level: String,
    pub requires_restart: bool,
}

/// Get current log level setting
#[tauri::command]
pub fn cmd_log_get_level(pool: State<DbPool>) -> Result<LogLevelResp, AppError> {
    let conn = pool
        .inner()
        .0
        .lock()
        .map_err(|e| AppError::Db(e.to_string()))?;

    let level = get_config_value(&conn, "log_level")
        .unwrap_or_else(|_| {
            if cfg!(debug_assertions) {
                "INFO".to_string()
            } else {
                "WARN".to_string()
            }
        })
        .to_uppercase();

    Ok(LogLevelResp {
        current_level: level,
        requires_restart: false,
    })
}

/// Set log level (requires app restart)
#[tauri::command]
pub fn cmd_log_set_level(pool: State<DbPool>, level: String) -> Result<String, AppError> {
    // 验证日志级别
    let valid_levels = ["OFF", "ERROR", "WARN", "INFO", "DEBUG", "TRACE"];
    let level_upper = level.to_uppercase();

    if !valid_levels.contains(&level_upper.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid log level: {}. Valid levels: OFF, ERROR, WARN, INFO, DEBUG, TRACE",
            level
        )));
    }

    let conn = pool
        .inner()
        .0
        .lock()
        .map_err(|e| AppError::Db(e.to_string()))?;

    // 保存到数据库
    conn.execute(
        "INSERT OR REPLACE INTO sync_config (key, value) VALUES ('log_level', ?1)",
        [&level_upper],
    )?;

    Ok(format!(
        "Log level set to {}. Please restart the application for changes to take effect.",
        level_upper
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_log_file_name_valid_base_files() {
        // 基础文件名应该通过
        assert!(validate_log_file_name("rust-default.log", "default").is_ok());
        assert!(validate_log_file_name("webview-default.log", "default").is_ok());
    }

    #[test]
    fn test_validate_log_file_name_valid_rotated_files() {
        // 轮转文件应该通过
        assert!(validate_log_file_name("rust-default.log.1", "default").is_ok());
        assert!(validate_log_file_name("rust-default.log.12", "default").is_ok());
        assert!(validate_log_file_name("webview-default.log.1", "default").is_ok());
        assert!(validate_log_file_name("webview-default.log.999", "default").is_ok());
    }

    #[test]
    fn test_validate_log_file_name_non_default_profile() {
        assert!(validate_log_file_name("rust-work.log", "work").is_ok());
        assert!(validate_log_file_name("webview-work.log", "work").is_ok());
        assert!(validate_log_file_name("rust-work.log.1", "work").is_ok());
        assert!(validate_log_file_name("webview-work.log.2", "work").is_ok());

        // Different profile file should be rejected.
        assert!(validate_log_file_name("rust-default.log", "work").is_err());
        assert!(validate_log_file_name("rust.log", "work").is_err());
    }

    #[test]
    fn test_validate_log_file_name_invalid_files() {
        // 其他文件应该被拒绝
        assert!(validate_log_file_name("other.log", "default").is_err());
        assert!(validate_log_file_name("rust", "default").is_err());
        assert!(validate_log_file_name("rust-default.log.txt", "default").is_err());
        assert!(validate_log_file_name("../etc/passwd", "default").is_err());
        assert!(validate_log_file_name("rust-default.log.abc", "default").is_err());
        assert!(validate_log_file_name("rust-default.log.", "default").is_err());
    }

    #[test]
    fn test_validate_log_file_name_path_traversal() {
        // 路径穿越应该被拒绝
        assert!(validate_log_file_name("../rust-default.log", "default").is_err());
        assert!(validate_log_file_name("../../rust-default.log", "default").is_err());
        assert!(validate_log_file_name("/etc/passwd", "default").is_err());
        assert!(validate_log_file_name("subdir/rust-default.log", "default").is_err());
    }

    #[test]
    fn test_redact_content() {
        let patterns = vec!["secret123".to_string(), "myAccessKey".to_string()];

        let input = "User logged in with secret123 and used myAccessKey to access S3";
        let expected = "User logged in with *** and used *** to access S3";
        let result = redact_content(input, &patterns);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_redact_content_multiple_occurrences() {
        let patterns = vec!["password".to_string()];

        let input = "First password is here, second password is there";
        let expected = "First *** is here, second *** is there";
        let result = redact_content(input, &patterns);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_redact_content_short_patterns_ignored() {
        // 小于 4 个字符的 pattern 应该被忽略（避免误遮罩）
        let patterns = vec!["abc".to_string()];
        let input = "This contains abc in it";
        let result = redact_content(input, &patterns);
        assert_eq!(result, input); // 应该没有变化
    }

    #[test]
    fn test_redact_content_no_patterns() {
        let patterns = vec![];
        let input = "Some sensitive content";
        let result = redact_content(input, &patterns);
        assert_eq!(result, input); // 应该没有变化
    }

    #[test]
    fn test_redact_content_empty_input() {
        let patterns = vec!["secret".to_string()];
        let input = "";
        let result = redact_content(input, &patterns);
        assert_eq!(result, "");
    }
}
