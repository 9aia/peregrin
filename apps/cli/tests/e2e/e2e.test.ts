import { describe, it, expect, vi, beforeEach, beforeAll, afterAll } from "vite-plus/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { Effect } from "effect";

const captured: Array<Effect.Effect<void, unknown, never>> = [];

vi.mock("handl-observer-otel", () => ({
  OtelObserver: class {
    constructor(_config: Record<string, unknown>) {}
  },
  OtelObserverLive: { _tag: "OtelObserverLive" },
}));

vi.mock("@effect/platform-node", async (importOriginal) => {
  const actual = (await importOriginal()) as Record<string, unknown>;
  return {
    ...actual,
    NodeRuntime: {
      ...(actual.NodeRuntime as Record<string, unknown>),
      runMain: (effect: Effect.Effect<void, unknown, never>) => {
        captured.push(effect);
      },
    },
  };
});

let tmpDir = "";

beforeAll(() => {
  tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "handl-cli-e2e-"));
});

beforeEach(() => {
  captured.length = 0;
  vi.resetModules();
});

afterAll(() => {
  if (tmpDir) {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

const writeWorkflow = (name: string, content: string): string => {
  const filePath = path.join(tmpDir, `${name}.yaml`);
  fs.writeFileSync(filePath, content, "utf8");
  return filePath;
};

describe("handl CLI e2e", () => {
  it("runs the version command", async () => {
    process.argv = ["node", "handl", "version"];
    await import("../../src/main.ts");
    expect(captured).toHaveLength(1);
    const log = vi.spyOn(console, "log").mockImplementation(() => {});
    await Effect.runPromise(captured[0]);
    expect(log).toHaveBeenCalledWith("0.0.0");
    log.mockRestore();
  });

  it("runs a workflow in dry-run mode using the in-memory runner", async () => {
    const workflowPath = writeWorkflow(
      "run",
      `name: e2e-run
nodes:
  - id: n1
    driver: noop
    on: [start]
    emits: [node.done]
`,
    );
    process.argv = ["node", "handl", "run", "--dry-run", workflowPath];
    await import("../../src/main.ts");
    expect(captured).toHaveLength(1);
    const log = vi.spyOn(console, "log").mockImplementation(() => {});
    await Effect.runPromise(captured[0]);
    const output = log.mock.calls.map((call) => String(call[0])).join("\n");
    expect(output).toContain("node.done");
    log.mockRestore();
  });

  it("validates a workflow", async () => {
    const workflowPath = writeWorkflow(
      "validate",
      `name: e2e-validate
nodes:
  - id: n1
    driver: noop
    on: [start]
    emits: [node.done]
`,
    );
    process.argv = ["node", "handl", "validate", workflowPath];
    await import("../../src/main.ts");
    expect(captured).toHaveLength(1);
    const log = vi.spyOn(console, "log").mockImplementation(() => {});
    await Effect.runPromise(captured[0]);
    const output = log.mock.calls.map((call) => String(call[0])).join("\n");
    expect(output).toContain("workflow: e2e-validate");
    expect(output).toContain("OK");
    log.mockRestore();
  });

  it("generates a root schema with stdout", async () => {
    const workflowPath = writeWorkflow(
      "typegen",
      `name: e2e-typegen
nodes: []
`,
    );
    process.argv = ["node", "handl", "typegen", "--stdout", "--no-handlers", workflowPath];
    await import("../../src/main.ts");
    expect(captured).toHaveLength(1);
    const log = vi.spyOn(console, "log").mockImplementation(() => {});
    await Effect.runPromise(captured[0]);
    const output = log.mock.calls.map((call) => String(call[0])).join("\n");
    const schema = JSON.parse(output);
    expect(schema.title).toContain("e2e-typegen");
    log.mockRestore();
  });
});
