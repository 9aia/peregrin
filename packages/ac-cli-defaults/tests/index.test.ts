import { describe, it, expect } from "vite-plus/test";
import { DefaultsLive, DefaultDriverRegistryLive } from "../src/index.ts";

describe("ac-cli-defaults", () => {
  it("exposes the default live layer", () => {
    expect(DefaultsLive).toBeDefined();
    expect(DefaultDriverRegistryLive).toBeDefined();
  });
});
