# AGENTS.md（给 AI 代码助手的项目协作指南）

本文件用于约束后续自动化/AI 代理在本仓库中的工作方式，确保实现与 `docs/PRD.md` 一致、可维护、可扩展。

## 项目定位
- **产品**：个人项目管理工具（Mac 本地）
- **核心能力**：项目状态机 + 状态时间线（不可变事件日志）、成员视图（做过/当前）、Partner（1:N，项目必须有且创建后不可变更）、Country
- **权威需求来源**：`docs/PRD.md`（任何行为/字段/约束变更必须先改 PRD）

## 技术栈（拍板）
- **Desktop**：Tauri（Rust）
- **Frontend**：React + TypeScript
- **Build**：Vite
- **UI**：Mantine
- **Validation**：zod（前端 DTO/表单输入预校验）
- **State**：zustand（建议）
- **DB**：SQLite（本地文件）
- **Rust DB**：rusqlite（同步 API，事务清晰）
- **Rust**：serde/serde_json、thiserror、uuid、chrono

## 总体架构（Clean Architecture）
```mermaid
flowchart TB
  UI[React/Vite + Mantine] -->|invoke| CMD[Tauri Commands (Rust)]
  CMD --> APP[Application UseCases (Tx)]
  APP --> DOMAIN[Domain Rules<br/>StatusMachine + Invariants]
  APP --> INFRA[SQLite Repos + Migrations + Export]
```

## 目录结构约定
> 若目录尚未生成，后续实现应按此结构落地。

```text
project-management/
  docs/
    PRD.md
  src/                       # Vite React frontend
    pages/
    components/
    stores/                  # zustand stores
    api/                     # typed invoke wrappers
    validators/              # zod schemas
  src-tauri/                 # Rust backend
    migrations/              # *.sql migrations
    src/
      commands/              # Tauri command handlers (DTO boundary)
      app/                   # use cases + transactions
      domain/                # entities + status machine + invariants
      repo/                  # traits (ports)
      infra/                 # sqlite impl + export
      main.rs
```

## 关键业务不变量（必须在 Rust 侧强制）
### Partner 约束
- 每个项目 **必须且只能关联 1 个** `partnerId`
- **项目创建后禁止变更 partnerId**
  - `project_update` 禁止携带 `partnerId` 字段：若出现则返回错误码 `PARTNER_IMMUTABLE`

### Owner 约束
- `ownerPersonId` 必须是该项目的**当前成员**
  - 设置/更换 owner 时：若不存在 active assignment（`endAt IS NULL`），需在同一事务内自动创建

### 状态机与时间线（不可变）
- 状态变更只能按 `docs/PRD.md` 的状态机跃迁
- 每次状态变更必须 **同一事务**完成：
  - insert `status_history`
  - update `projects.current_status`（以及 `updated_at`/`archived_at`）
- `status_history` 为不可变事件日志：禁止编辑/删除（纠错通过追加说明或追加新事件）
- 特殊跃迁必须 `note`（见 PRD）

### Assignment 约束
- 同一 `(projectId, personId)` 不允许存在两条 active assignment（`endAt IS NULL`）
- 结束参与时若无 active assignment，返回 `ASSIGNMENT_NOT_ACTIVE`

## 命令层 API 契约（实现必须对齐）
- 命令、DTO、错误码定义以 `docs/PRD.md` 的 **13.9 节**为准
- **错误码稳定性**：前端仅依赖 `code` 分支逻辑；`message/details` 用于展示与调试
- 建议支持 `ifMatchUpdatedAt`（乐观锁）以避免 UI 并发覆盖（即便是单机也可能多窗口/多操作）

## SQLite 迁移策略（必须）
- 在 Rust 侧启动时执行 migrations（建议 `BEGIN IMMEDIATE`）
- 使用 `schema_migrations(version, applied_at)` 记录已应用版本
- 迁移失败必须回滚并阻止继续运行（避免半迁移损坏）

## 开发运行（约定命令）
> 具体脚本在项目脚手架生成后应保持一致。

- **安装依赖**：
  - `npm install`
- **本地开发（推荐）**：
  - `npm run dev`（前端）
  - `cargo tauri dev`（带桌面壳）
- **构建**：
  - `cargo tauri build`

## 代码风格与工程规范
- **命名**：
  - Rust：`snake_case`（函数/模块），`PascalCase`（类型），错误类型 `*Error`
  - TS：变量/函数 `camelCase`，类型/组件 `PascalCase`
- **注释语言**：
  - 标准库/常规代码注释：English
  - 复杂业务规则/不变量说明：中文（解释“为什么”）
- **边界分层**：
  - UI 不直接拼 SQL
  - Commands 只做 DTO 映射与权限/参数最小校验（无账号体系时主要是输入校验）
  - UseCase 负责事务与业务规则编排
  - Domain 只放纯规则（状态机/不变量判断），不依赖 IO

## 变更规则（重要）
- 任何改变字段、状态机、错误码、命令契约，都必须同步更新 `docs/PRD.md`
- 任何新增表/字段，都必须提供 migration，并更新 PRD 的数据模型章节
- 如需添加“项目非状态字段的审计日志”（例如以后允许更换 Partner），必须先在 PRD 的未来扩展/范围中明确


---

## UI 设计系统（现代化规范）

### 主题配置
项目采用 **毛玻璃/渐变风格**（Arc / Raycast 参考），配置文件位于 `src/theme.ts`。

#### 核心配置
- **主色调**: Indigo (#6366f1) / Violet (#8b5cf6) 渐变系
- **圆角**: 统一使用 `md` (8px) 和 `lg` (12px)
- **阴影**: 柔和阴影体系，透明度范围 0.05-0.12
  ```typescript
  shadows: {
    xs: '0 1px 3px rgba(0, 0, 0, 0.05)',
    sm: '0 2px 8px rgba(0, 0, 0, 0.06)',
    md: '0 4px 12px rgba(0, 0, 0, 0.08)',
    lg: '0 8px 24px rgba(0, 0, 0, 0.10)',
    xl: '0 16px 48px rgba(0, 0, 0, 0.12)',
  }
  ```
- **字体**: Inter + 系统字体栈
  ```typescript
  fontFamily: 'Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif'
  ```

#### 毛玻璃效果
```css
/* 导航栏/顶栏 */
backgroundColor: 'rgba(255, 255, 255, 0.7)',
backdropFilter: 'blur(12px)',
borderBottom: '1px solid rgba(0, 0, 0, 0.06)'
```

#### 渐变背景
- **页面背景** (`src/index.css`):
  ```css
  background: linear-gradient(135deg, #f8f9ff 0%, #f0f2ff 50%, #faf8ff 100%);
  ```
- **英雄卡片** (详情页顶部):
  ```typescript
  background: 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%)'
  ```

#### 状态色彩映射
配置位置: `src/utils/statusColor.ts`
```typescript
PROJECT_STATUS_COLORS = {
  Planning: 'gray',
  Active: 'green',
  'On Hold': 'yellow',
  Completed: 'blue',
  Cancelled: 'red',
}
```

### 组件设计模式

#### 列表页规范
文件示例: `ProjectsList.tsx`, `PeopleList.tsx`, `PartnersList.tsx`

**必须包含**:
- 标题使用 `Title order={3}`
- 主操作按钮使用渐变样式:
  ```typescript
  <Button
    variant="gradient"
    gradient={{ from: 'indigo', to: 'violet' }}
    leftSection={<IconPlus size={18} />}
  >
    新建XXX
  </Button>
  ```
- 筛选区包裹在 `Paper` 卡片中（带阴影）
- 表格使用 `Table.ScrollContainer` + `striped="even"` + `highlightOnHover`
- 状态徽章使用 `getProjectStatusColor()` 统一色彩映射

#### 详情页规范
文件示例: `ProjectDetail.tsx`, `PersonDetail.tsx`, `PartnerDetail.tsx`

**必须包含**:
- **英雄卡片**: 顶部渐变背景展示核心信息
  ```typescript
  <Paper style={{
    background: 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%)',
    color: 'white',
  }}>
    <Title order={2}>{name}</Title>
    <Badge variant="filled" style={{ backgroundColor: 'rgba(255,255,255,0.25)' }}>
      {status}
    </Badge>
  </Paper>
  ```
- 信息卡片使用 `Paper`（自动带阴影，不使用 `withBorder`）
- 返回按钮: `<Button variant="subtle" leftSection={<IconArrowLeft size={16} />}>`
- 编辑按钮: `<Button variant="light" leftSection={<IconEdit size={16} />}>`
- 表格必须包裹在 `Table.ScrollContainer` 中

#### 表单页规范
文件示例: `ProjectForm.tsx`, `PersonForm.tsx`, `PartnerForm.tsx`

**必须包含**:
- 整个表单包裹在 `Paper` 卡片中
- 表单标题 `Title order={3}`
- 提交按钮使用渐变样式:
  ```typescript
  <Button
    variant="gradient"
    gradient={{ from: 'indigo', to: 'violet' }}
    leftSection={<IconDeviceFloppy size={18} />}
  >
    {isEdit ? '保存' : '创建'}
  </Button>
  ```
- 返回按钮使用 `variant="subtle"`
- 双列响应式布局: `<SimpleGrid cols={{ base: 1, sm: 2 }}>`

### TypeScript 规范

#### 类型导入（强制）
项目启用了 `verbatimModuleSyntax`，**必须**使用 `type` 关键字导入纯类型：

```typescript
// ✅ 正确
import { createTheme, type MantineColorsTuple } from '@mantine/core';
import type { MantineColor } from '@mantine/core';

// ❌ 错误（会导致白屏）
import { MantineColorsTuple } from '@mantine/core';
import { MantineColor } from '@mantine/core';
```

**原因**: 避免运行时导入纯类型，减小打包体积，防止编译失败。

### 图标使用规范

#### 图标库
使用 `@tabler/icons-react`（已安装）

#### 常用图标映射
| 场景 | 图标 | 尺寸 |
|------|------|------|
| 项目 | `IconFolder` | 20 (导航), 18 (按钮) |
| 成员 | `IconUsers` | 20 (导航), 18 (按钮) |
| 合作方 | `IconBuildingCommunity` | 20 (导航), 18 (按钮) |
| 设置 | `IconSettings` | 20 (导航), 18 (按钮) |
| 返回 | `IconArrowLeft` | 16 |
| 编辑 | `IconEdit` | 16 |
| 添加 | `IconPlus` | 18 |
| 保存 | `IconDeviceFloppy` | 18 |
| 品牌/应用 | `IconBriefcase` | 24 |

#### 使用方式
```typescript
import { IconFolder } from '@tabler/icons-react';

// 按钮中使用
<Button leftSection={<IconFolder size={18} />}>
  按钮文字
</Button>

// 导航中使用
<NavLink
  leftSection={<IconFolder size={20} stroke={1.5} />}
  label="项目"
/>
```

### 响应式布局规范

#### Mantine 断点
- `base`: 默认（移动端，0px+）
- `sm`: 768px+
- `md`: 992px+
- `lg`: 1200px+

#### 常用模式
```typescript
// AppShell 导航栏
navbar={{ width: { base: 200, md: 220 }, breakpoint: 'sm' }}
padding={{ base: 'xs', sm: 'md' }}

// 表单双列布局
<SimpleGrid cols={{ base: 1, sm: 2 }} spacing="md">
  <TextInput label="字段1" />
  <TextInput label="字段2" />
</SimpleGrid>

// Flex 自动换行
<Flex wrap="wrap" gap="xs" justify="space-between">
  {/* 按钮或筛选项 */}
</Flex>

// 表格水平滚动（必须）
<Table.ScrollContainer minWidth={400}>
  <Table>
    {/* 表格内容 */}
  </Table>
</Table.ScrollContainer>
```

#### Tauri 窗口最小尺寸
`src-tauri/tauri.conf.json`:
```json
{
  "minWidth": 800,
  "minHeight": 500
}
```
**原因**: 保证侧栏（200px）+ 主内容区（~600px）始终可见，且大于 Mantine `sm` 断点。

### 文件组织规范

#### 主题相关
- `src/theme.ts` - Mantine 主题配置（`createTheme`）
- `src/index.css` - 全局样式（背景渐变、滚动条美化）

#### 工具函数
- `src/utils/statusColor.ts` - 状态色彩映射 (`getProjectStatusColor`)
- `src/utils/errorToast.ts` - 错误/成功提示封装

#### 常量
- `src/constants/countries.ts` - 国家列表、项目状态常量

### 常见问题解决方案

#### 应用白屏
**症状**: Tauri 窗口打开后显示白屏，开发者工具可能显示编译错误

**原因**: TypeScript 类型导入错误导致编译失败

**解决步骤**:
1. 检查终端输出或运行 `npm run build` 查看编译错误
2. 搜索 `TS1484` 错误（`must be imported using a type-only import`）
3. 修复类型导入：添加 `type` 关键字
4. Vite 会自动热更新（HMR）

**示例修复**:
```typescript
// 修复前
import { MantineColorsTuple } from '@mantine/core';

// 修复后
import { type MantineColorsTuple } from '@mantine/core';
// 或
import type { MantineColorsTuple } from '@mantine/core';
```

#### 端口占用（5173 被占用）
**解决**:
```bash
lsof -ti:5173 | xargs kill -9
npm run tauri dev
```

#### 样式不生效
**检查清单**:
1. CSS 导入顺序是否正确（`main.tsx`）:
   ```typescript
   import '@mantine/core/styles.css';
   import '@mantine/dates/styles.css';
   import '@mantine/notifications/styles.css';
   import './index.css'; // 自定义样式放最后
   ```
2. 主题是否正确传入 `MantineProvider`:
   ```typescript
   import { theme } from './theme';
   <MantineProvider theme={theme}>
   ```
3. 组件是否使用了正确的 Mantine props（检查 v7 文档）

#### 滚动条问题
**症状**: 页面或表格无法滚动，或出现双滚动条

**解决**:
- `body` 必须设置 `overflow: hidden`（由 AppShell 内部滚动）
- `AppShell.Main` 设置 `style={{ overflow: 'auto' }}`
- 表格必须包裹在 `Table.ScrollContainer` 中

### UI 改造流程（标准步骤）

#### 1. 规划阶段
- 确定设计风格（简洁/毛玻璃/企业级）
- 列出需要改造的页面和组件
- 明确优先级（建议顺序：Layout → 列表 → 详情 → 表单）

#### 2. 主题配置
- 创建 `src/theme.ts`
- 配置主色、圆角、阴影、字体
- 设置组件默认属性（`defaultProps` + `styles`）

#### 3. 全局样式
- 更新 `src/index.css`
- 设置背景渐变
- 美化滚动条（WebKit）

#### 4. 安装依赖
```bash
npm install @tabler/icons-react
```

#### 5. 组件改造顺序
1. **Layout** (`Layout.tsx`): 导航栏/顶栏毛玻璃效果 + 图标
2. **列表页**: 筛选卡片 + 渐变按钮 + 表格样式
3. **详情页**: 英雄卡片 + 信息卡片 + 响应式
4. **表单页**: 卡片容器 + 渐变提交按钮

#### 6. 统一管理
- 创建 `utils/statusColor.ts` 统一色彩映射
- 确保所有图标引用一致
- 检查响应式断点使用

#### 7. 测试验证
- 编译检查: `npm run build`（必须无错误）
- Lint 检查: 使用 `ReadLints` 工具
- 不同窗口尺寸测试（拖拽缩小到最小值）
- 检查所有页面的导航和交互

#### 8. 调整窗口尺寸
根据实际布局需求调整 `src-tauri/tauri.conf.json` 中的 `minWidth` 和 `minHeight`，确保内容不会消失。
