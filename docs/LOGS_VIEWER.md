# 应用日志查看功能说明

## 概述

Projex 内置了完整的日志查看功能，用户可以在应用内直接查看前端和后端的日志，便于：
- **本地调试**：开发时快速定位问题
- **用户反馈**：用户遇到问题时可截图或复制日志协助排查
- **系统监控**：查看应用运行状态、错误记录

## 核心特性

### 1. 自动日志管理
- **分离记录**：前端日志（`webview.log`）与后端日志（`rust.log`）分开存储
- **智能轮转**：单个文件最大 10MB，超过后自动创建新文件（如 `rust.log.1`）
- **空间控制**：仅保留最近 5 个文件，自动清理旧日志
- **级别控制**：
  - Debug 模式：Info 级别（详细信息）
  - Release 模式：Warn 级别（仅警告和错误）

### 2. 安全脱敏
- **默认开启**：查看日志时自动遮罩敏感信息
- **脱敏内容**：
  - S3 Access Key
  - S3 Secret Key
  - 其他可能的凭据信息
- **可选关闭**：用户可手动关闭脱敏（用于完整排查）

### 3. 高效浏览
- **分页加载**：默认加载最新 256KB（约 3000 行）
- **向前翻页**：点击"加载更多"查看更早的日志
- **实时搜索**：输入关键词即时高亮匹配内容
- **自动刷新**：可选每 2 秒自动更新（查看实时日志）

### 4. 便捷操作
- **一键复制**：将日志内容复制到剪贴板
- **下载保存**：导出日志为 `.log` 或 `.txt` 文件
- **清空日志**：清除当前日志文件内容（带确认对话框）

## 使用指南

### 访问日志查看器

#### 方式一：通过设置页面
1. 打开应用，点击左侧导航栏的 **设置（Settings）**
2. 找到"应用日志"区块
3. 点击 **查看日志** 按钮

#### 方式二：直接访问（开发时）
在浏览器地址栏输入：`http://localhost:5173/logs`

### 查看日志

#### 1. 选择日志文件
顶部有文件选择器，可以切换：
- **rust.log** - 后端日志（Rust 代码、数据库操作、S3 同步等）
- **webview.log** - 前端日志（React 组件、用户操作、UI 事件等）

#### 2. 浏览日志内容
- 日志按时间倒序显示（最新的在上）
- 等宽字体显示，便于阅读
- 滚动区域固定高度，不影响页面其他部分

#### 3. 加载更早的日志
- 日志顶部有"加载更多"按钮
- 点击后向前加载更多内容（每次 256KB）
- 如果没有更多数据，按钮会消失

### 搜索日志

1. 在搜索框输入关键词（如 "error"、"sync"、"failed"）
2. 匹配的内容会自动高亮显示
3. 搜索仅对当前已加载的内容生效
4. 清空搜索框恢复完整显示

### 自动刷新

适用于监控实时日志：
1. 打开"自动刷新"开关
2. 日志会每 2 秒自动重新加载
3. 适合查看正在进行的同步、导入等操作

### 脱敏开关

- **默认开启**：所有敏感信息显示为 `***`
- **关闭脱敏**：完整显示所有内容（需要时使用）
- **注意**：关闭脱敏后截图或复制时注意保护隐私

### 导出日志

#### 复制到剪贴板
1. 点击"复制"按钮
2. 当前显示的日志内容会复制到剪贴板
3. 可粘贴到邮件、工单系统等

#### 下载为文件
1. 点击"下载"按钮
2. 选择保存位置和文件名
3. 日志会保存为 `.log` 或 `.txt` 文件

### 清空日志

1. 点击"清空"按钮（红色）
2. 确认对话框会提示操作不可撤销
3. 确认后当前日志文件内容会被清空
4. 文件本身保留，后续日志继续写入

### 调整日志级别

日志级别控制记录哪些日志：

| 级别 | 记录内容 | 推荐场景 |
|------|---------|---------|
| **ERROR** | 仅错误 | 生产环境，只关注严重问题 |
| **WARN** | 警告 + 错误 | 默认，平衡信息量与噪音 |
| **INFO** | 信息 + 警告 + 错误 | 调试业务流程 |
| **DEBUG** | 调试 + 信息 + 警告 + 错误 | 深度排查，信息量最大 |

**设置步骤**：
1. 在日志页面顶部找到"日志级别"下拉框
2. 选择新的级别（如从 WARN 改为 INFO）
3. 系统提示"需要重启"
4. 关闭并重新打开应用
5. 新的日志级别生效

**注意**：
- 级别变更会立即保存到数据库
- 但必须重启应用才能生效（tauri-plugin-log 启动时初始化）
- 不影响已有日志文件，仅影响后续记录

## 使用场景

### 场景 1：开发调试

**问题**：S3 同步失败

**步骤**：
1. 进入日志查看器
2. 选择 `rust.log`（后端日志）
3. 搜索 "sync" 或 "S3"
4. 查看错误信息，定位问题原因

### 场景 2：用户反馈问题

**问题**：用户报告"项目创建失败"

**步骤**：
1. 让用户打开日志查看器
2. 搜索 "project" 或 "error"
3. 复制相关日志内容
4. 发送给开发团队分析

### 场景 3：监控实时操作

**问题**：查看大批量导入的进度

**步骤**：
1. 开始导入操作
2. 打开日志查看器，选择 `rust.log`
3. 开启"自动刷新"
4. 实时查看导入进度和可能的错误

### 场景 4：性能分析

**问题**：应用响应慢，需要分析

**步骤**：
1. 清空日志（避免旧数据干扰）
2. 执行慢操作
3. 查看日志中的时间戳
4. 分析哪个环节耗时最长

## 技术细节

### 日志文件位置

日志文件存储在系统的应用数据目录：

- **macOS**：`~/Library/Application Support/com.nickdu.projex/logs/`
- **Windows**：`%APPDATA%\com.nickdu.projex\logs\`
- **Linux**：`~/.local/share/com.nickdu.projex/logs/`

### 日志轮转规则

```
rust.log          # 当前日志文件（最新）
rust.log.1        # 第一次轮转（上一个文件）
rust.log.2        # 第二次轮转
rust.log.3        # 第三次轮转
rust.log.4        # 第四次轮转（最旧的保留文件）
```

当 `rust.log` 达到 10MB 时：
1. 删除 `rust.log.4`（如果存在）
2. 重命名 `rust.log.3` → `rust.log.4`
3. 重命名 `rust.log.2` → `rust.log.3`
4. 重命名 `rust.log.1` → `rust.log.2`
5. 重命名 `rust.log` → `rust.log.1`
6. 创建新的 `rust.log`

### 安全限制

#### 白名单机制
后端仅允许读取以下文件：
- `rust.log` 及其轮转文件（`rust.log.1` ~ `rust.log.4`）
- `webview.log` 及其轮转文件（`webview.log.1` ~ `webview.log.4`）

拒绝访问：
- 系统文件（如 `/etc/passwd`）
- 父目录文件（如 `../config.json`）
- 其他应用的日志

#### 资源限制
- 单次读取最大 2MB（防止内存溢出）
- 默认读取 256KB（平衡性能与信息量）
- 分页加载（避免一次性加载大文件）

### 脱敏算法

```rust
fn redact_content(content: &str, patterns: &[String]) -> String {
    let mut result = content.to_string();
    for pattern in patterns {
        if pattern.len() >= 4 {  // 忽略过短的 pattern
            result = result.replace(pattern, "***");
        }
    }
    result
}
```

从数据库 `sync_config` 表读取：
- `s3_access_key`
- `s3_secret_key`

替换所有出现的位置为 `***`。

## 常见问题

### Q1：为什么看不到日志文件？

**可能原因**：
1. 应用刚启动，还没有产生日志
2. Release 模式下只记录 Warn 及以上级别

**解决方法**：
1. 执行一些操作（如创建项目、同步）生成日志
2. 刷新日志文件列表
3. 如果仍然没有，检查应用数据目录

### Q2：日志文件为什么是空的？

**可能原因**：
1. 刚执行了"清空"操作
2. 应用重启后还没有新日志

**解决方法**：
1. 执行一些操作生成日志
2. 等待自动刷新或手动刷新

### Q3：搜索找不到关键词？

**可能原因**：
1. 关键词在更早的日志中（未加载）
2. 关键词被脱敏遮罩了

**解决方法**：
1. 点击"加载更多"加载更多日志
2. 尝试关闭脱敏开关再搜索
3. 下载完整日志文件用文本编辑器搜索

### Q4：日志太多，如何快速定位问题？

**建议**：
1. 使用搜索功能，搜索 "error" 或 "failed"
2. 查看日志时间戳，定位问题发生时间
3. 清空日志后重现问题，减少干扰信息

### Q5：如何确保隐私安全？

**最佳实践**：
1. 保持脱敏开关开启
2. 分享日志前检查是否有个人信息
3. 不要在公开渠道发布未脱敏的日志
4. 必要时手动编辑敏感内容后再分享

## 开发者参考

### 前端日志输出

使用统一的 `logger` 抽象层：

```typescript
import { logger } from '../utils/logger';

// 信息日志
logger.info('Sync completed:', result);

// 错误日志
logger.error('Sync failed:', error);

// 调试日志（仅 debug 模式）
logger.debug('Current state:', state);

// 警告日志
logger.warn('Deprecated API called');
```

**禁止**直接使用 `console.log/warn/error/debug`。

### 后端日志输出

使用标准的 `log` crate：

```rust
use log::{debug, info, warn, error};

// 信息日志
info!("Sync completed: device_id={}", device_id);

// 错误日志
error!("Sync failed: {}", err);

// 调试日志（仅 debug 模式）
debug!("Current state: {:?}", state);

// 警告日志
warn!("Deprecated API called");
```

### 添加新的脱敏规则

编辑 `src-tauri/src/commands/logs.rs` 中的 `get_redaction_patterns` 函数：

```rust
fn get_redaction_patterns(conn: &Connection) -> Vec<String> {
    let mut patterns = Vec::new();

    // 现有规则
    if let Ok(key) = get_config_value(conn, "s3_access_key") {
        patterns.push(key.trim().to_string());
    }
    if let Ok(key) = get_config_value(conn, "s3_secret_key") {
        patterns.push(key.trim().to_string());
    }

    // 添加新规则（示例）
    if let Ok(token) = get_config_value(conn, "api_token") {
        patterns.push(token.trim().to_string());
    }

    patterns
}
```

### 调整日志级别

编辑 `src-tauri/src/lib.rs` 中的日志配置：

```rust
let log_level = if cfg!(debug_assertions) {
    log::LevelFilter::Info  // 改为 Debug 查看更多信息
} else {
    log::LevelFilter::Warn  // 改为 Info 记录更多信息
};
```

### 调整文件轮转参数

```rust
tauri_plugin_log::Builder::default()
    .level(log_level)
    .max_file_size(20 * 1024 * 1024)  // 改为 20MB
    .targets([...])
    .build()
```

## 总结

Projex 的日志查看功能提供了：
- ✅ 实时查看前后端日志
- ✅ 智能脱敏保护隐私
- ✅ 高效搜索与浏览
- ✅ 便捷导出与分享
- ✅ 安全的白名单访问控制

适用于开发调试、用户反馈、系统监控等多种场景，是 Projex 可观测性的重要组成部分。
