import { describe, it, expect } from "vite-plus/test";
import { matches, captureWildcards } from "../src/index.ts";

describe("handl-engine", () => {
  it("matches exact patterns", () => {
    expect(matches("a.b", "a.b")).toBe(true);
    expect(matches("a.b", "a.c")).toBe(false);
  });

  it("matches single wildcards", () => {
    expect(matches("a.*", "a.b")).toBe(true);
    expect(matches("a.*", "a.b.c")).toBe(false);
  });

  it("matches double wildcards", () => {
    expect(matches("a.**", "a.b")).toBe(true);
    expect(matches("a.**.c", "a.x.y.c")).toBe(true);
  });

  it("captures wildcard values", () => {
    const caps = captureWildcards("a.*.c", "a.b.c");
    expect(caps).not.toBeNull();
    expect(caps!["1"]).toBe("b");
  });
});
