import { describe, it, expect } from "vite-plus/test";
import { StreamingDriver } from "../src/index.ts";

describe("ac-driver-streaming", () => {
  it("exports a driver named streaming", () => {
    expect(StreamingDriver.name).toBe("streaming");
  });
});
