import { describe, it, expect } from "vite-plus/test";
import { Effect } from "effect";
import {
  CapabilityChecker,
  REDACTED,
  redactPayload,
  buildValidator,
  LoadBreaker,
  ResourceGate,
  makePreexec,
  validateObject,
} from "../src/index.ts";
import { Event, EventSchema } from "ac-core";

describe("ac-safety", () => {
  it("redacts secret strings from payloads", () => {
    const redacted = redactPayload({ key: "secret", text: "hello" }, { key: "secret" });
    expect(redacted).toEqual({ key: REDACTED, text: "hello" });
  });

  it("CapabilityChecker allows a permitted shell command", () => {
    const checker = new CapabilityChecker({
      default_policy: "deny",
      shell: { allow: ["ls"] },
    });
    expect(checker.check("shell", { cmd: "ls" })).toEqual([]);
  });

  it("CapabilityChecker denies a non-allowed shell command", () => {
    const checker = new CapabilityChecker({
      default_policy: "deny",
      shell: { allow: ["ls"] },
    });
    const violations = checker.check("shell", { cmd: "rm" });
    expect(violations.length).toBeGreaterThan(0);
  });

  it("CapabilityChecker filters secrets by allow list", () => {
    const checker = new CapabilityChecker({
      default_policy: "deny",
      secrets: ["A"],
    });
    const filtered = checker.filterSecrets({ A: "one", B: "two" });
    expect(filtered).toEqual({ A: "one" });
  });

  it("buildValidator returns null when no schemas have constraints", () => {
    expect(buildValidator([new EventSchema("empty")])).toBeNull();
  });

  it("SchemaValidator reports missing required and type errors", () => {
    const validator = buildValidator([
      new EventSchema("user", { name: "string", age: "integer?" }),
    ])!;
    const missing = validator.validate(new Event("user", {}));
    expect(missing.some((e) => e.includes("missing required property"))).toBe(true);
    const wrongType = validator.validate(new Event("user", { name: 5 }));
    expect(wrongType.some((e) => e.includes("expected type string"))).toBe(true);
  });

  it("LoadBreaker trips when load exceeds threshold", () => {
    const breaker = new LoadBreaker({ cpu_threshold: 1, sustained_seconds: 0 }, () => [
      100, 0, 1024,
    ]);
    breaker.update(0); // seed the overload timestamp
    expect(breaker.update(0)).toBe(true);
  });

  it("ResourceGate caps and releases weight", async () => {
    const gate = new ResourceGate(1);
    const weight = await Effect.runPromise(gate.acquire(5));
    expect(weight).toBe(1);
    gate.release(1);
    expect(gate.isOverloaded()).toBe(false);
  });

  it("makePreexec returns null with no limits", () => {
    expect(makePreexec()).toBeNull();
  });

  it("validateObject reports missing required and type errors", () => {
    const schema = new EventSchema("input", { cmd: "string", timeout: "integer?" });
    const missing = validateObject(schema, {});
    expect(missing.some((e) => e.includes("cmd") && e.includes("missing required"))).toBe(true);
    const wrongType = validateObject(schema, { cmd: "ls", timeout: "fast" });
    expect(
      wrongType.some((e) => e.includes("timeout") && e.includes("expected type integer")),
    ).toBe(true);
  });

  it("validateObject returns empty for an empty schema", () => {
    const schema = new EventSchema("input");
    expect(validateObject(schema, { anything: 1 })).toEqual([]);
  });
});
