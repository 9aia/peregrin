import { expect, it } from "vite-plus/test";
import { fn } from "../src/index.ts";

it("fn returns the expected greeting", () => {
  expect(fn()).toBe("Hello, tsdown!");
});
