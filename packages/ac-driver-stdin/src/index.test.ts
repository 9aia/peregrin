import { describe, it, expect } from "vite-plus/test";
import { StdinDriver } from "./index.js";

describe("handl-driver-stdin", () => {
  it("is named stdin", () => {
    expect(StdinDriver.name).toBe("stdin");
  });
});
