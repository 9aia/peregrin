import { describe, it, expect } from "vite-plus/test";
import { StreamingDriver, StreamingDriverLive } from "../src/index.ts";

describe("handl-driver-streaming", () => {
  it("exports a driver named streaming", () => {
    expect(StreamingDriver.name).toBe("streaming");
  });

  it("exports a Live layer", () => {
    expect(StreamingDriverLive).toBeDefined();
  });
});
