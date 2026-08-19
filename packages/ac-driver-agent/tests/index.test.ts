import { describe, it, expect } from "vite-plus/test";
import { AgentDriver, AgentDriverLive } from "../src/index.ts";

describe("ac-driver-agent", () => {
  it("exports a driver named agent", () => {
    expect(AgentDriver.name).toBe("agent");
  });

  it("exports a Live layer", () => {
    expect(AgentDriverLive).toBeDefined();
  });
});
