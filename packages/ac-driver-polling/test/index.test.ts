import { expect, test } from "vite-plus/test";
import { PollingDriver } from "../src/index.ts";

test("ac-driver-polling", () => {
  expect(PollingDriver.name).toBe("polling");
});
