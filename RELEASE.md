# Projex v1.0.0 - Release Notes

## Downloads

### Binary artifacts (built via GitHub Actions CI)
- **macOS (Apple Silicon)**: `Projex_1.0.0_aarch64.dmg`
- **macOS (Intel)**: `Projex_1.0.0_x64.dmg`
- **Windows**: `Projex_1.0.0_x64-setup.exe` / `Projex_1.0.0_x64_en-US.msi`

### Data directory
- **macOS**: `~/Library/Application Support/com.nickdu.projex/app.db`
- **Windows**: `%APPDATA%/com.nickdu.projex/app.db`
- Auto-created on first launch; migrations run automatically.

---

## Completed Milestones (M1–M6)

### M1: Data Loop
- Project CRUD, status transitions, list view
- Partner / Person / Assignment management
- Status machine validation + immutable status history

### M2: Full UI
- Project detail (info + status timeline + member management)
- Person detail (current & historical projects)
- Partner detail (associated projects)
- Multi-dimension filtering (status, country, partner, owner, member, tags)
- Sorting (updated_at, priority, due_date) + server-side pagination

### M3: Deliverable
- JSON export with `save` dialog
- macOS `.app` / `.dmg` packaging
- Empty-state guidance, confirmation dialogs, Toast error handling

### M4: Sync & Improvements
- S3 multi-device sync (Delta + Snapshot + Vector Clock)
- Compatible with AWS S3, Cloudflare R2, MinIO
- JSON import (idempotent, `INSERT OR IGNORE`)
- Tag-based filtering, Zustand global stores

### M5: Internationalization (i18n)
- `i18next` + `react-i18next` (default: English, fallback: English)
- ~300 translation keys (en.json + zh.json)
- Runtime language switch (Settings page)
- Country names via `i18n-iso-countries`, role labels via i18n keys

### M6: Rich Text Comments
- Tiptap editor (StarterKit, Link, Image, Table, TaskList, Mention, Slash commands)
- Comment CRUD, pin/unpin, associate person
- Integrated into Project Detail page
- Export/import schema upgraded to v2 (includes comments)

---

## Key Features

### Project Management
- **Status Machine**: BACKLOG → PLANNED → IN_PROGRESS → BLOCKED → DONE → ARCHIVED
- **Immutable Timeline**: every status change is an append-only event log
- **Multi-dimension Filter**: status, country, partner, owner, member, tags
- **Sorting**: updated_at, priority, due_date
- **Tags**: free-form tag system with multi-select filtering

### People & Partners
- Person: name, email, role, note, active/inactive
- Partner: name, note, immutable binding to project after creation
- Person detail: current projects + historical projects

### Rich Text Comments
- Tiptap-powered rich text: headings, lists, task lists, tables, images, code blocks
- Slash commands (`/`) + @mention for quick formatting and person references
- Pin important comments to top

### Data Management
- **Local SQLite**: fully private, no cloud dependency
- **JSON Export/Import**: full backup with schema v2 (idempotent import)
- **S3 Sync**: delta-based sync with vector clock conflict resolution
- **Snapshot**: full backup/restore to S3 with gzip + SHA-256 checksum

### UI/UX
- Frosted glass (backdrop-filter) + gradient buttons (indigo → violet)
- Hero card on detail pages
- i18n: English / 中文, runtime switchable
- Responsive layout (min window 800×500)

---

## Tech Stack

### Frontend
- **Framework**: React 19 + TypeScript 5
- **Build**: Vite 7
- **UI**: Mantine 7
- **Rich Text**: Tiptap + @mantine/tiptap
- **State**: Zustand
- **i18n**: i18next + react-i18next
- **Routing**: React Router DOM 7

### Backend (Rust / Tauri)
- **Desktop**: Tauri v2
- **Database**: SQLite (rusqlite 0.32)
- **Serialization**: serde + serde_json
- **Error Handling**: thiserror
- **ID**: uuid v4
- **Time**: chrono (ISO-8601)
- **Sync**: aws-sdk-s3, vector clock, flate2, sha2

### Architecture
- **Clean Architecture**: Domain → Application → Infrastructure
- **Status Machine**: strict transition rules enforced in Rust domain
- **Transactions**: ACID guarantees for critical operations
- **Type Safety**: full-stack TypeScript + Rust

---

## Quick Start

### Development
```bash
npm install
npm run tauri dev
```

### Production Build
```bash
npm run tauri build
```

### First Use
1. Open the DMG / installer
2. Drag `Projex.app` to Applications (macOS) or run the installer (Windows)
3. Launch the app — database is auto-initialized
4. Create a Partner → Create People → Create a Project

---

## Data Schema (v2)

Exported JSON structure:
```json
{
  "schemaVersion": 2,
  "exportedAt": "2026-02-11T00:00:00.000Z",
  "persons": [...],
  "partners": [...],
  "projects": [...],
  "assignments": [...],
  "statusHistory": [...],
  "comments": [...]
}
```

---

## Known Issues

- ⚠️ JS bundle ~1.2 MB (gzip ~381 KB) — code-splitting planned for future
- ⚠️ S3 sync delta download/apply is partially implemented (snapshot sync fully works)

---

## Documentation

- **Product Requirements**: `docs/PRD.md`
- **Milestones**: `docs/MILESTONES.md`
- **S3 Sync Design**: `docs/SYNC_S3_DESIGN.md`
- **Sync Explained**: `docs/SYNC_EXPLAINED.md`
- **Agent Guide**: `AGENTS.md`

---

## Build Info

- **Version**: 1.0.0
- **Date**: 2026-02-11
- **Platforms**: macOS (Apple Silicon + Intel), Windows
- **Rust Tests**: 245 passed (13 test files)
- **Frontend**: ESLint 0 errors, TypeScript strict mode
