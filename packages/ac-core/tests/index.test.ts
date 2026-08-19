import { describe, it, expect } from "vite-plus/test";
import { Event } from "ac-core";

describe("ac-core", () => {
  it("exports Event", () => {
    const event = new Event("test");
    expect(event.type).toBe("test");
  });
});
