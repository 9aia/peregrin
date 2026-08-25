import { describe, it, expect } from "vite-plus/test";
import { Effect } from "effect";
import { JsonStore } from "../src/index.ts";
import { Event } from "handl-core";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

describe("handl-store-json", () => {
  it("appends and retrieves events", async () => {
    const tmp = path.join(os.tmpdir(), `handl-store-json-${Date.now()}.jsonl`);
    try {
      const store = await Effect.runPromise(JsonStore.make(tmp));
      store.append(new Event("x"));
      expect(store.latest()?.type).toBe("x");
      store.clear();
      expect(store.latest()).toBeNull();
    } finally {
      fs.rmSync(tmp, { force: true });
    }
  });
});
