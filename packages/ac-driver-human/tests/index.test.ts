import { describe, it, expect } from "vite-plus/test";
import { HumanDriver, HumanDriverLive } from "../src/index.ts";

describe("ac-driver-human", () => {
  it("exports a driver named human", () => {
    expect(HumanDriver.name).toBe("human");
  });

  it("exports a Live layer", () => {
    expect(HumanDriverLive).toBeDefined();
  });
});
