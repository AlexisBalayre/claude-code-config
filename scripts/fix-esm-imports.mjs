#!/usr/bin/env node

/**
 * Post-build script: adds .js extensions to relative imports in compiled output.
 *
 * Source files use extensionless imports (clean for tsx + drizzle-kit),
 * but Node.js ESM requires explicit .js extensions at runtime.
 * This script patches the compiled dist/ files after tsc.
 *
 * Usage: node scripts/fix-esm-imports.mjs
 * Runs from the package root (uses process.cwd()/dist).
 */

import { existsSync } from "node:fs";
import { readFile, readdir, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";

const DIST_DIR = resolve(process.cwd(), "dist");

// Matches: from "./foo" or from "../bar/baz" (but not from "./foo.js" or from "drizzle-orm")
const RELATIVE_IMPORT_RE = /((?:from|import)\s+["'])(\.\.?\/[^"']+?)(["'])/g;

async function fixFile(filePath) {
  const content = await readFile(filePath, "utf-8");
  const fileDir = dirname(filePath);
  const fixed = content.replace(RELATIVE_IMPORT_RE, (match, prefix, path, suffix) => {
    if (path.endsWith(".js") || path.endsWith(".json")) return match;
    // Check if the path resolves to a directory with an index.js
    const resolved = resolve(fileDir, path);
    if (existsSync(join(resolved, "index.js"))) {
      return `${prefix}${path}/index.js${suffix}`;
    }
    return `${prefix}${path}.js${suffix}`;
  });
  if (fixed !== content) {
    await writeFile(filePath, fixed);
  }
}

async function walk(dir) {
  const entries = await readdir(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = join(dir, entry.name);
    if (entry.isDirectory()) {
      await walk(fullPath);
    } else if (entry.name.endsWith(".js")) {
      await fixFile(fullPath);
    }
  }
}

await walk(DIST_DIR);
