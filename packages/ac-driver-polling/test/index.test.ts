import { expect, test } from "vite-plus/test";
import { PollingDriver } from "../src/index.ts";

test("handl-driver-polling", () => {
  expect(PollingDriver.name).toBe("polling");
});
