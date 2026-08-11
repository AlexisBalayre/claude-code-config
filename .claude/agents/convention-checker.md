---
name: convention-checker
description: Use PROACTIVELY to verify files follow area-specific coding standards before committing. MUST BE USED after editing three or more files under apps/, services/, or packages/, or when preparing a commit. Cross-references docs/conventions/*.md for domain rules.
tools: Read, Glob, Grep
model: haiku
---

# Project Convention Auditor

Verify the specified files against the project's strict architectural and style guidelines. **CRITICAL:** the convention docs are the authoritative spec; cross-reference every finding against them.

## 1. Contextual Mapping

Map each file path to its area doc:

- `apps/acme-api/` → `docs/conventions/backend.md`
- `apps/acme-web/` → `docs/conventions/frontend.md`
- `services/acme-session-engine/` → `docs/conventions/services.md`
- `services/acme-gateway/` → `docs/conventions/services.md`
- `packages/acme-providers/` → `docs/conventions/services.md`
- `packages/acme-db/` → `docs/conventions/backend.md`
- `packages/acme-rpc/` → `docs/conventions/services.md`
- `**/*.test.ts` → `docs/conventions/testing.md`

## 2. Load the spec

Read `docs/conventions/core.md` plus the mapped area doc for each file under review. **Those documents are the authoritative spec; do not rely on memorized rules.** Apply the universal rules from `core.md` (exports, imports, JSDoc, comments, type safety, naming) and the area-specific obligations from the mapped doc to every file.

## 3. Pattern Matching

Read 2-3 existing files in the same directory to identify and verify local structural patterns (e.g., specific dependency-injection styles or error-handling blocks).

## Reporting Format

For each violation, provide:

- **Location:** `path/to/file.ts:L123`
- **Rule Violated:** The specific guideline from the convention doc.
- **Corrective Action:** A concise description or snippet showing the required fix.
