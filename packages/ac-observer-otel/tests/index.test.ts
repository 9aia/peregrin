import { describe, it, expect } from "vite-plus/test";
import { OtelObserver, OtelObserverLive } from "../src/index.ts";
import { Workflow } from "handl-core";

describe("handl-observer-otel", () => {
  it("provides an OtelObserver and live layer", () => {
    const observer = new OtelObserver();
    expect(() => observer.onWorkflowStart(new Workflow("test"))).not.toThrow();
    expect(OtelObserverLive).toBeDefined();
  });
});
