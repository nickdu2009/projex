# 项目管理工具 Project Management

一款现代化的本地项目管理工具，基于 Tauri + React 构建，提供流畅的用户体验和强大的项目管理功能。

![](https://img.shields.io/badge/Platform-macOS-blue.svg)
![](https://img.shields.io/badge/License-MIT-green.svg)
![](https://img.shields.io/badge/Version-1.0.0-orange.svg)

## ✨ 核心特性

### 项目管理
- 📋 **完整的项目生命周期管理**：从规划到归档的完整流程
- 🔄 **固定状态机**：BACKLOG → PLANNED → IN_PROGRESS → BLOCKED → DONE → ARCHIVED
- 🏷️ **多维度筛选**：按状态、国家、合作方、负责人、参与成员筛选
- 📊 **灵活排序**：支持按更新时间、优先级、截止日期排序
- 🗂️ **标签系统**：自定义标签管理项目

### 成员管理
- 👥 **成员信息管理**：姓名、邮箱、角色、备注
- 🎭 **预设角色**：测试、产品经理、后端开发、前端开发
- 📈 **项目视图**：当前参与项目、历史项目

### 合作方管理
- 🏢 **合作方档案**：名称、备注、关联项目
- 🔒 **不可变约束**：项目创建后 Partner 不可更改

### 数据管理
- 💾 **本地 SQLite 数据库**：所有数据存储在本地
- 📤 **JSON 导出**：支持导出所有数据用于备份
- 🔐 **数据安全**：无云同步，数据完全私有

### UI/UX
- 🎨 **现代化设计**：毛玻璃效果、渐变按钮、英雄卡片
- 🌈 **主题系统**：自定义 Mantine 主题
- 📱 **响应式布局**：适配不同窗口大小
- 🎯 **空状态引导**：友好的空状态提示
- ⚠️ **二次确认**：关键操作确认对话框

## 🛠️ 技术栈

| 层级 | 技术 |
|------|------|
| **桌面框架** | Tauri v2 |
| **前端框架** | React 18 + TypeScript |
| **构建工具** | Vite 7 |
| **UI 组件库** | Mantine 7 |
| **图标** | Tabler Icons |
| **后端语言** | Rust |
| **数据库** | SQLite (rusqlite) |
| **状态管理** | React Hooks |
| **路由** | React Router DOM |

## 📋 前置要求

- **Node.js** 18+ ([下载](https://nodejs.org/))
- **Rust** 1.77.2+ ([安装](https://rustup.rs/))
- **macOS** 12+ (当前仅支持 macOS)

## 🚀 快速开始

### 开发模式

```bash
# 1. 克隆仓库
git clone <repository-url>
cd project-management

# 2. 安装依赖
npm install

# 3. 启动开发服务器
npm run tauri dev
```

首次运行会自动：
- 创建数据目录：`~/Library/Application Support/com.tauri.dev/`
- 初始化数据库：`app.db`
- 执行数据库迁移

### 生产构建

```bash
# 构建应用
npm run tauri build

# 构建产物位于：src-tauri/target/release/bundle/
```

## 📖 使用指南

### 1. 创建合作方
1. 导航到"合作方"页面
2. 点击"新建合作方"
3. 输入名称和备注
4. 点击"创建"

### 2. 创建成员
1. 导航到"成员"页面
2. 点击"新建成员"
3. 输入姓名、邮箱、角色、备注
4. 点击"创建"

### 3. 创建项目
1. 导航到"项目"页面
2. 点击"新建项目"
3. 填写必填信息：
   - 项目名称
   - 国家（东亚/东南亚）
   - 合作方（必须先创建）
   - 负责人（必须先创建）
4. 可选填写：描述、优先级、日期、标签
5. 点击"创建"

### 4. 管理项目
- **变更状态**：项目详情页 → 变更状态按钮
- **添加成员**：项目详情页 → 成员区 → 选择成员 → 加入
- **编辑项目**：项目详情页 → 编辑按钮

### 5. 导出数据
1. 导航到"设置"页面
2. 点击"导出数据"
3. 选择保存位置
4. 确认保存

## 📁 项目结构

```
project-management/
├── docs/                    # 文档
│   ├── PRD.md              # 产品需求文档
│   └── MILESTONES.md       # 里程碑计划
├── src/                     # 前端代码
│   ├── api/                # API 调用封装
│   ├── components/         # React 组件
│   ├── constants/          # 常量定义
│   ├── pages/              # 页面组件
│   ├── utils/              # 工具函数
│   ├── theme.ts            # Mantine 主题
│   └── main.tsx            # 入口文件
├── src-tauri/              # 后端代码
│   ├── migrations/         # 数据库迁移
│   ├── src/
│   │   ├── app/           # 业务逻辑层
│   │   ├── commands/      # Tauri 命令
│   │   ├── domain/        # 领域层（状态机）
│   │   ├── infra/         # 基础设施层（数据库）
│   │   └── error.rs       # 错误处理
│   ├── Cargo.toml         # Rust 依赖
│   └── tauri.conf.json    # Tauri 配置
├── AGENTS.md               # AI Agent 规范
└── README.md               # 本文件
```

## 🔧 配置说明

### 数据库位置
- **macOS**: `~/Library/Application Support/com.tauri.dev/app.db`

### 窗口尺寸
- **默认**: 1200x800
- **最小**: 800x500

### 数据 Schema 版本
- **当前版本**: 1
- 支持向后兼容的迁移机制

## 🤝 贡献指南

详见 `AGENTS.md` 了解：
- 代码规范
- 架构设计
- UI 设计系统
- 常见问题解决方案

## 📄 许可证

MIT License

## 🆘 常见问题

### Q: 如何重置数据库？
A: 删除 `~/Library/Application Support/com.tauri.dev/app.db` 文件，重启应用。

### Q: 如何备份数据？
A: 使用"设置"页面的"导出数据"功能，将数据导出为 JSON 文件。

### Q: 端口 5173 被占用？
A: 运行 `lsof -ti:5173 | xargs kill -9` 释放端口。

### Q: 编译失败？
A: 确保 Rust 和 Node.js 版本符合要求，尝试 `cargo clean` 和 `npm clean-install`。

## 📮 反馈与支持

遇到问题或有建议？请创建 Issue 或查看 `docs/PRD.md` 了解更多细节。
