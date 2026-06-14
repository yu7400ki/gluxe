import { expect, test } from "vitest";

import { STYLE_PROP_SAMPLES } from "./style-prop-samples";

// The fixture is consumed by the Rust conformance test
// `every_ts_style_prop_is_recognized` in crates/core/src/style/parse.rs.
// After editing STYLE_PROP_SAMPLES, regenerate with:
//   pnpm -C packages/react test:run -- -u
test("style prop samples fixture is in sync", async () => {
  await expect(`${JSON.stringify(STYLE_PROP_SAMPLES, null, 2)}\n`).toMatchFileSnapshot(
    "../../../crates/core/tests/fixtures/style_prop_samples.json",
  );
});
