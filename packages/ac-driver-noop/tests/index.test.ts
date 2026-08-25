import { describe, it, expect } from "vite-plus/test";
import { NoopDriver, NoopDriverLive } from "../src/index.ts";

describe("handl-driver-noop", () => {
  it("exports a driver named noop", () => {
    expect(new NoopDriver().name).toBe("noop");
  });

  it("exports a Live layer", () => {
    expect(NoopDriverLive).toBeDefined();
  });
});
