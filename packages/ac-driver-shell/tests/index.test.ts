import { describe, it, expect } from "vite-plus/test";
import { ShellDriver, ShellDriverLive } from "../src/index.ts";

describe("handl-driver-shell", () => {
  it("exports a driver named shell", () => {
    expect(ShellDriver.name).toBe("shell");
  });

  it("exports a Live layer", () => {
    expect(ShellDriverLive).toBeDefined();
  });
});
