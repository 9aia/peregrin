import { describe, it, expect } from "vite-plus/test";
import { PythonDriver, PythonDriverLive } from "../src/index.ts";

describe("ac-driver-python", () => {
  it("exports a driver named python", () => {
    expect(PythonDriver.name).toBe("python");
  });

  it("exports a Live layer", () => {
    expect(PythonDriverLive).toBeDefined();
  });
});
