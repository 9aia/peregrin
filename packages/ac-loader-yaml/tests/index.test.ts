import { describe, it, expect } from "vite-plus/test";
import { Effect } from "effect";
import { loadWorkflowFromText } from "../src/index.ts";

describe("ac-loader-yaml", () => {
  it("loads a minimal workflow from text", async () => {
    const wf = await Effect.runPromise(loadWorkflowFromText("name: test\nnodes: []"));
    expect(wf.name).toBe("test");
    expect(wf.nodes).toHaveLength(0);
  });

  it("fails when nodes is not an array", async () => {
    await expect(
      Effect.runPromise(loadWorkflowFromText("name: test\nnodes: notArray")),
    ).rejects.toBeDefined();
  });
});
