import { describe, it, expect } from "vite-plus/test";
import { PollingDriver, PollingDriverLive } from "../src/index.ts";

describe("handl-driver-polling", () => {
  it("exports a driver named polling", () => {
    expect(PollingDriver.name).toBe("polling");
  });

  it("exports a Live layer", () => {
    expect(PollingDriverLive).toBeDefined();
  });
});
