import { describe, it, expect } from "vite-plus/test";
import { Effect } from "effect";
import { resolveTemplate } from "../src/index.ts";
import type { ExecutionContext } from "ac-core";

describe("ac-template-engine", () => {
  const ctx: ExecutionContext = {
    config: { name: "world" },
    secrets: {},
    log: [],
    emit: () => {},
    inject: () => {},
    dryRun: false,
    idempotencyKey: "1",
    traceparent: "",
    state: { get: () => undefined, set: () => {}, toRecord: () => ({}) },
    fires: 0,
    requestShutdown: () => {},
    getSecret: () => "",
    strict: false,
    capabilities: {},
    workflowCapabilities: {},
    resources: {},
    missingSecrets: [],
  };

  it("passes through text without placeholders", async () => {
    const text = await Effect.runPromise(resolveTemplate("hello world", [], ctx));
    expect(text).toBe("hello world");
  });

  it("leaves unknown placeholders unchanged", async () => {
    const text = await Effect.runPromise(resolveTemplate("${notknown}", [], ctx));
    expect(text).toBe("${notknown}");
  });
});
