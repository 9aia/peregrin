import { it, expect } from "@effect/vitest";
import { vi } from "vite-plus/test";
import { Effect, Either } from "effect";
import { Node } from "ac-core";
import type { ExecutionContext } from "ac-core";
import { AppriseDriver } from "../src/index.ts";

const { mockSend } = vi.hoisted(() => ({ mockSend: vi.fn() }));
vi.mock("@toanalien/ahha", () => ({
  send: mockSend,
  registerService: vi.fn(),
}));
vi.mock("apprise", () => ({ Apprise: vi.fn() }));

const makeContext = (secrets: Record<string, string> = {}): ExecutionContext => ({
  config: {},
  secrets,
  log: [],
  emit: () => undefined,
  inject: () => undefined,
  dryRun: false,
  idempotencyKey: "test-key",
  traceparent: "",
  state: {
    get: () => undefined,
    set: () => undefined,
    clear: () => undefined,
    close: () => undefined,
    toRecord: () => ({}),
  },
  fires: 0,
  requestShutdown: () => undefined,
  getSecret: (name: string, default_?: string) => secrets[name] ?? default_ ?? "",
  strict: false,
  capabilities: {},
  workflowCapabilities: {},
  resources: {},
  missingSecrets: [],
});

const fakeContext = makeContext();

const setHttpFetch = (status = 200) => {
  const fetchMock = vi.fn().mockResolvedValue(new Response("ok", { status }));
  globalThis.fetch = fetchMock as unknown as typeof globalThis.fetch;
  return fetchMock;
};

const setNoopFetch = () => {
  globalThis.fetch = vi.fn() as unknown as typeof globalThis.fetch;
};

it.effect("sends a notification and emits notify.sent", () =>
  Effect.gen(function* () {
    const fetchMock = setHttpFetch(200);

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

it.effect("notify:send emits notify.failed when the target is missing", () =>
  Effect.gen(function* () {
    const node = new Node("alert", "apprise", "send", [], ["notified", "notify.failed"], "any", {
      body: "test",
    });

    const result = yield* Effect.either(AppriseDriver.invoke(node, [], fakeContext));

    expect(Either.isLeft(result)).toBe(true);
    if (Either.isLeft(result)) {
      expect(result.left.reason).toContain("missing 'params.target'");
    }
  }),
);

it.effect("notify:send emits notify.failed on a non-ok HTTP response", () =>
  Effect.gen(function* () {
    setHttpFetch(401);

    const node = new Node("alert", "apprise", "send", [], ["notify.sent", "notify.failed"], "any", {
      target: "http://example.com/notify",
      body: "test",
    });

    const events = yield* AppriseDriver.invoke(node, [], fakeContext);

    expect(events[0].type).toBe("notify.failed");
    expect(events[0].payload.success).toBe(false);
    expect(events[0].payload.error).toContain("status 401");
  }),
);

it.effect("notify:send emits notify.failed when apprise returns false", () =>
  Effect.gen(function* () {
    setNoopFetch();
    mockSend.mockReset();
    mockSend.mockResolvedValue({ success: false, error: { message: "bad token" } });

    const node = new Node("alert", "apprise", "send", [], ["notify.sent", "notify.failed"], "any", {
      target: "slack://bad-token/#channel",
      body: "test",
    });

    const events = yield* AppriseDriver.invoke(node, [], fakeContext);

    expect(events[0].type).toBe("notify.failed");
    expect(events[0].payload.success).toBe(false);
    expect(events[0].payload.error).toContain("bad token");
  }),
);

it.effect("notify:send emits notify.failed on apprise exception", () =>
  Effect.gen(function* () {
    setNoopFetch();
    mockSend.mockReset();
    mockSend.mockRejectedValue(new Error("network down"));

    const node = new Node("alert", "apprise", "send", [], ["notify.sent", "notify.failed"], "any", {
      target: "slack://token/#channel",
      body: "test",
    });

    const events = yield* AppriseDriver.invoke(node, [], fakeContext);

    expect(events[0].type).toBe("notify.failed");
    expect(events[0].payload.success).toBe(false);
    expect(events[0].payload.error).toContain("network down");
  }),
);

it.effect("notify:send_multi sends to multiple targets", () =>
  Effect.gen(function* () {
    setNoopFetch();
    mockSend.mockReset();
    mockSend.mockResolvedValue({ success: true });

    const node = new Node("broadcast", "apprise", "send_multi", [], ["notified"], "any", {
      targets: ["slack://token/#channel1", "tgram://bottoken/chatid"],
      title: "Broadcast",
      body: "Multi-target message",
    });

    const events = yield* AppriseDriver.invoke(node, [], fakeContext);

    expect(events[0].type).toBe("notified");
    expect(events[0].payload.success).toBe(true);
    expect(events[0].payload.count).toBe(2);
    expect(mockSend).toHaveBeenCalledTimes(2);
    expect(mockSend).toHaveBeenCalledWith("slack://token/#channel1", "Multi-target message", {
      title: "Broadcast",
      format: "text",
    });
    expect(mockSend).toHaveBeenCalledWith("telegram://bottoken/chatid", "Multi-target message", {
      title: "Broadcast",
      format: "text",
    });
  }),
);

it.effect("notify:send_multi converts a single string target", () =>
  Effect.gen(function* () {
    setNoopFetch();
    mockSend.mockReset();
    mockSend.mockResolvedValue({ success: true });

    const node = new Node("broadcast", "apprise", "send_multi", [], ["notified"], "any", {
      targets: "slack://token/#channel",
      body: "test",
    });

    const events = yield* AppriseDriver.invoke(node, [], fakeContext);

    expect(events[0].payload.count).toBe(1);
    expect(events[0].payload.success).toBe(true);
    expect(mockSend).toHaveBeenCalledTimes(1);
  }),
);

it.effect("notify:send_multi fails when targets are missing", () =>
  Effect.gen(function* () {
    const node = new Node("broadcast", "apprise", "send_multi", [], ["notified"], "any", {
      body: "test",
    });

    const result = yield* Effect.either(AppriseDriver.invoke(node, [], fakeContext));

    expect(Either.isLeft(result)).toBe(true);
    if (Either.isLeft(result)) {
      expect(result.left.reason).toContain("missing 'params.targets'");
    }
  }),
);

it.effect("notify:send_multi emits notify.failed when one target fails", () =>
  Effect.gen(function* () {
    setNoopFetch();
    mockSend.mockReset();
    mockSend.mockImplementation((target: string) => {
      if (target.includes("bad")) {
        return Promise.resolve({ success: false, error: { message: "bad token" } });
      }
      return Promise.resolve({ success: true });
    });

    const node = new Node(
      "broadcast",
      "apprise",
      "send_multi",
      [],
      ["notified", "notify.failed"],
      "any",
      {
        targets: ["slack://good-token/#channel", "slack://bad-token/#channel"],
        body: "test",
      },
    );

    const events = yield* AppriseDriver.invoke(node, [], fakeContext);

    expect(events[0].type).toBe("notify.failed");
    expect(events[0].payload.success).toBe(false);
    expect(events[0].payload.count).toBe(2);
  }),
);
