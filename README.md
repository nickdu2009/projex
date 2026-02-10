# Projex

一款现代化的本地项目管理工具，基于 Tauri + React 构建，支持 S3 多设备同步。

![](https://img.shields.io/badge/Platform-macOS-blue.svg)
![](https://img.shields.io/badge/License-MIT-green.svg)
![](https://img.shields.io/badge/Version-1.0.0-orange.svg)
![](https://img.shields.io/badge/Tests-230%20passed-brightgreen.svg)

## 核心特性

### 项目管理
- 完整的项目生命周期管理：BACKLOG → PLANNED → IN_PROGRESS → BLOCKED → DONE → ARCHIVED
- 不可变状态时间线（事件日志）
- 多维度筛选：状态、国家、合作方、负责人、参与成员、标签
- 灵活排序：更新时间、优先级、截止日期
- 分页列表（服务端分页）

### 成员管理
- 成员信息：姓名、邮箱、角色、备注
- 预设角色：测试、产品经理、后端开发、前端开发
- 项目视图：当前参与项目、历史项目

### 合作方管理
- 合作方档案：名称、备注、关联项目
- 不可变约束：项目创建后 Partner 不可更改

### 数据管理
- 本地 SQLite 数据库，数据完全私有
- JSON 导出 / 导入（幂等，重复 ID 自动跳过）
- S3 多设备同步（兼容 AWS S3 / Cloudflare R2 / MinIO）
- 全量快照备份与恢复

### UI/UX
- 毛玻璃效果、渐变按钮、英雄卡片
- Zustand 全局状态管理
- 响应式布局，适配不同窗口大小
- 空状态引导、关键操作二次确认

## 技术栈

| 层级 | 技术 |
|------|------|
| **桌面框架** | Tauri v2 |
| **前端** | React 19 + TypeScript |
| **构建工具** | Vite 7 |
| **UI 组件库** | Mantine 7 |
| **状态管理** | Zustand |
| **后端** | Rust |
| **数据库** | SQLite (rusqlite) |
| **同步** | aws-sdk-s3 + Vector Clock |

## 前置要求

- **Node.js** 18+
- **Rust** 1.77.2+
- **macOS** 12+

## 快速开始

```bash
# 安装依赖
npm install

# 开发模式
npm run tauri dev

# 生产构建
npm run tauri build

# 运行后端测试（230 个测试用例）
cd src-tauri && cargo test
```

首次运行会自动创建数据目录和数据库，并执行迁移。

## 项目结构

```
projex/
├── docs/
│   ├── PRD.md                # 产品需求文档
│   ├── MILESTONES.md         # 里程碑跟踪
│   ├── SYNC_S3_DESIGN.md     # S3 同步架构设计
│   └── SYNC_EXPLAINED.md     # 同步机制说明
├── src/                       # 前端
│   ├── api/                  # Tauri invoke 封装
│   ├── components/           # 共享组件 (ConfirmModal, EmptyState, SyncStatusBar)
│   ├── stores/               # Zustand stores
│   ├── sync/                 # 前端同步管理
│   ├── pages/                # 页面组件
│   └── theme.ts              # Mantine 主题
├── src-tauri/                 # 后端 (Rust)
│   ├── migrations/           # SQL 迁移 (3 个)
│   ├── tests/                # 集成测试 (12 个文件, 230 个用例)
│   └── src/
│       ├── app/              # 业务逻辑 (CRUD, 导入导出)
│       ├── commands/         # Tauri 命令层
│       ├── domain/           # 领域 (状态机)
│       ├── infra/            # 基础设施 (DB)
│       └── sync/             # S3 同步 (Delta, Snapshot, VectorClock)
├── AGENTS.md                  # AI Agent 协作规范
└── README.md
```

## 使用指南

1. **创建合作方** → "合作方" → "新建合作方"
2. **创建成员** → "成员" → "新建成员"
3. **创建项目** → "项目" → "新建项目"（需先创建合作方和成员）
4. **管理项目** → 项目详情页变更状态、添加/移除成员
5. **导出/导入数据** → "设置" → "导出数据" / "导入数据"
6. **配置 S3 同步** → "设置" → S3 配置 → 保存 → 立即同步

## 配置说明

| 项目 | 值 |
|------|---|
| 数据库位置 | `~/Library/Application Support/com.nickdu.projex/app.db` |
| 默认窗口 | 1200 x 800 |
| 最小窗口 | 800 x 500 |
| Schema 版本 | 1 |

## 许可证

MIT License
