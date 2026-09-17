import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

// Self-contained (no shared factory): this package rides along in repos that
// are not pnpm workspaces. Coverage is not thresholded here; the logic worth
// gating lives in the pure utils, and the `.script.ts` entrypoints are the
// argv/env/`gh` shell around it, where coverage would mean mocking GitHub.
export default defineConfig({
  resolve: {
    alias: {
      "@src": fileURLToPath(new URL("./src", import.meta.url)),
      "@test": fileURLToPath(new URL("./test", import.meta.url)),
    },
  },
  test: {
    globals: true,
    environment: "node",
    include: ["test/**/*.test.ts"],
  },
});
