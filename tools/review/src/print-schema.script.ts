import { z } from "zod";
import { reviewSummarySchema } from "./review-metrics.schemas";

/**
 * Emits the review structured-output contract as JSON Schema on stdout, for the
 * workflow to inject into claude-code-action's `--json-schema`. Generating it
 * here from the zod source each run is what keeps the schema the model is held to
 * from drifting away from `review-metrics.schemas.ts`.
 */
function main(): void {
  const schema = z.toJSONSchema(reviewSummarySchema) as Record<string, unknown>;
  // The action wants a bare schema object; a top-level $schema key carries no
  // instruction for the model and trips some consumers.
  delete schema.$schema;
  process.stdout.write(JSON.stringify(schema));
}

main();
