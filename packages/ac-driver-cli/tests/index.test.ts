import { expect, test } from "vite-plus/test";
import { CliDriver } from "../src/index.ts";

test("CliDriver is named 'cli'", () => {
  expect(CliDriver.name).toBe("cli");
});
