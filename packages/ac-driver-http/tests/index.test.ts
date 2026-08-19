import { describe, it, expect } from "vite-plus/test";
import { HttpDriver, HttpDriverLive } from "../src/index.ts";

describe("ac-driver-http", () => {
  it("exports a driver named http", () => {
    expect(HttpDriver.name).toBe("http");
  });

  it("exports a Live layer", () => {
    expect(HttpDriverLive).toBeDefined();
  });
});
