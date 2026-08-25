import { describe, it, expect } from "vite-plus/test";
import { CliDriver, CliDriverLive } from "../src/index.ts";

describe("handl-driver-cli", () => {
  it("exports a driver named cli", () => {
    expect(CliDriver.name).toBe("cli");
  });

  it("exports a Live layer", () => {
    expect(CliDriverLive).toBeDefined();
  });
});
