import { describe, it, expect } from "vite-plus/test";
import { MemoryStore } from "../src/index.ts";
import { Event } from "handl-core";

describe("handl-store-memory", () => {
  it("stores and retrieves events by type", () => {
    const store = new MemoryStore();
    store.append(new Event("a", { n: 1 }));
    store.append(new Event("b"));
    store.append(new Event("a", { n: 2 }));
    expect(store.all()).toHaveLength(3);
    expect(store.byType("a")).toHaveLength(2);
    expect([...store.seenTypes()]).toContain("a");
    expect(store.latest()?.type).toBe("a");
    store.clear();
    expect(store.all()).toHaveLength(0);
  });
});
