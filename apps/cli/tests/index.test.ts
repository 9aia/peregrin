import { describe, it, expect, vi, beforeEach } from "vite-plus/test";
import fs from "node:fs";
import path from "node:path";
import { Effect } from "effect";
import { Workflow } from "handl-core";
import { DriverRegistry } from "handl-engine";
import {
  makeDefaultsLive,
  DefaultDriverRegistryLive,
  DefaultsLive,
  MetricsObserver,
  NoopObserverLayer,
} from "handl-cli-defaults";

vi.mock("handl-observer-otel", () => ({
  OtelObserver: class {
    constructor(_config: Record<string, unknown>) {}
  },
  OtelObserverLive: { _tag: "OtelObserverLive" },
}));

const captured: Array<Effect.Effect<void, unknown, never>> = [];

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

beforeEach(() => {
  captured.length = 0;
  vi.resetModules();
});

it("exposes a cli bin entry", () => {
  const pkg = JSON.parse(
    fs.readFileSync(path.join(import.meta.dirname, "../package.json"), "utf8"),
  );
  expect(pkg.bin.handl).toContain("cli.mjs");
});

describe("handl CLI argument parsing", () => {
  it("runs the version command", async () => {
    process.argv = ["node", "handl", "version"];
    await import("../src/main.ts");
    expect(captured).toHaveLength(1);
    const log = vi.spyOn(console, "log").mockImplementation(() => {});
    await Effect.runPromise(captured[0]);
    expect(log).toHaveBeenCalledWith("0.0.0");
    log.mockRestore();
  });

  it("rejects an unknown subcommand", async () => {
    process.argv = ["node", "handl", "foobar"];
    await import("../src/main.ts");
    expect(captured).toHaveLength(1);
    await expect(Effect.runPromise(captured[0])).rejects.toBeDefined();
  });
});

describe("handl-cli-defaults default config loading", () => {
  it("exposes the default live layer", () => {
    expect(DefaultsLive).toBeDefined();
    expect(DefaultDriverRegistryLive).toBeDefined();
  });

  it("makeDefaultsLive returns a layer for an observer", () => {
    const layer = makeDefaultsLive(NoopObserverLayer);
    expect(layer).toBeDefined();
  });

  it("resolves the noop driver from the default registry", async () => {
    const program = Effect.gen(function* () {
      const registry = yield* DriverRegistry;
      const driver = yield* registry.resolve("noop");
      return driver.name;
    });
    const name = await Effect.runPromise(Effect.provide(program, DefaultDriverRegistryLive));
    expect(name).toBe("noop");
  });

  it("emits Prometheus text from the MetricsObserver", () => {
    const observer = new MetricsObserver({ port: 0 });
    const log = vi.spyOn(console, "log").mockImplementation(() => {});
    observer.onWorkflowStart(new Workflow());
    observer.onWorkflowEnd(0, false, false, false);
    expect(log).toHaveBeenCalled();
    expect(log.mock.calls[0][0]).toContain("ac_workflow_status");
    log.mockRestore();
  });

  it("does not start an HTTP server when the metrics port is 0", () => {
    const observer = new MetricsObserver({ port: 0 });
    const log = vi.spyOn(console, "log").mockImplementation(() => {});
    observer.onWorkflowStart(new Workflow());
    expect(log).not.toHaveBeenCalled();
    log.mockRestore();
  });
});
