import { describe, it, expect } from "vite-plus/test";
import { StdinDriver, StdinDriverLive } from "../src/index.ts";

describe("ac-driver-stdin", () => {
  it("exports a driver named stdin", () => {
    expect(StdinDriver.name).toBe("stdin");
  });

  it("exports a Live layer", () => {
    expect(StdinDriverLive).toBeDefined();
  });
});
