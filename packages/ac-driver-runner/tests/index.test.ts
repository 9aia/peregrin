import { describe, it, expect } from "vite-plus/test";
import { RunnerDriver, RunnerDriverLive } from "../src/index.ts";

describe("ac-driver-runner", () => {
  it("exports a driver named runner", () => {
    expect(RunnerDriver.name).toBe("runner");
  });

  it("exports a Live layer", () => {
    expect(RunnerDriverLive).toBeDefined();
  });
});
