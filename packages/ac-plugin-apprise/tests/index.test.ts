import { it, expect } from "@effect/vitest";
import { vi } from "vite-plus/test";
import { Effect } from "effect";
import { Node } from "ac-core";
import type { ExecutionContext } from "ac-core";
import { AppriseDriver } from "../src/index.ts";

const fakeContext: ExecutionContext = {
  config: {},
  secrets: {},
  log: [],
  emit: () => undefined,
  inject: () => undefined,
  dryRun: false,
  idempotencyKey: "test-key",
  traceparent: "",
  state: {
    get: () => undefined,
    set: () => undefined,
    toRecord: () => ({}),
  },
  fires: 0,
  requestShutdown: () => undefined,
  getSecret: () => "",
  strict: false,
  capabilities: {},
  workflowCapabilities: {},
  resources: {},
  missingSecrets: [],
};

it.effect("sends a notification and emits notify.sent", () =>
  Effect.gen(function* () {
    const fetchMock = vi.fn().mockResolvedValue(new Response("ok", { status: 200 }));
    globalThis.fetch = fetchMock as unknown as typeof globalThis.fetch;

    const node = new Node(
      "test-apprise",
      "apprise",
      "send",
      [],
      ["notify.sent", "notify.failed"],
      "any",
      {
        target: "http://example.com/notify",
        body: "hello",
        title: "title",
      },
    );

    const events = yield* AppriseDriver.invoke(node, [], fakeContext);

    expect(events.length).toBe(1);
    expect(events[0].type).toBe("notify.sent");
    expect(events[0].payload.success).toBe(true);
    expect(events[0].payload.target).toBe("http://example.com/notify");
    expect(fetchMock).toHaveBeenCalledTimes(1);
  }),
);
