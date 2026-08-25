import { describe, it, expect } from "vite-plus/test";
import { NoopObserver, Live, Layer } from "../src/index.ts";
import { Workflow } from "handl-core";

describe("handl-observer-noop", () => {
  it("provides a NoopObserver and layers", () => {
    const observer = new NoopObserver();
    expect(observer.onWorkflowStart(new Workflow())).toBeUndefined();
    expect(Live).toBeDefined();
    expect(Layer).toBeDefined();
  });
});
