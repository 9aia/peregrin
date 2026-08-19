import { describe, it, expect } from "vite-plus/test";
import { Effect } from "effect";
import { discover } from "../src/index.ts";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

describe("ac-file-router", () => {
  it("discovers .node.yml files", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-file-router-"));
    try {
      fs.writeFileSync(path.join(tmp, "demo.node.yml"), "");
      const result = await Effect.runPromise(discover(tmp));
      expect(result.nodes).toHaveLength(1);
      expect(result.nodes[0].name).toBe("demo");
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("fails for a missing base directory", async () => {
    await expect(
      Effect.runPromise(discover("/nonexistent/path/should/fail")),
    ).rejects.toBeDefined();
  });
});
