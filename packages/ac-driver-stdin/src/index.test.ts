import { describe, it, expect } from "vite-plus/test";
import { StdinDriver } from "./index.js";

describe("ac-driver-stdin", () => {
  it("is named stdin", () => {
    expect(StdinDriver.name).toBe("stdin");
  });
});
