import { describe, it, expect } from "vite-plus/test";
import { FakeDriver } from "../src/index.ts";
import { Event } from "handl-core";

describe("handl-testing-library", () => {
  it("FakeDriver returns its configured events", () => {
    const driver = new FakeDriver("test", [new Event("y")]);
    expect(driver.name).toBe("test");
    expect(driver.events).toHaveLength(1);
  });
});
