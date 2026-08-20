import { it, expect } from "@effect/vitest";
import { vi } from "vite-plus/test";
import { Effect, Either } from "effect";
import { Node } from "ac-core";
import type { ExecutionContext } from "ac-core";
import { LiteLLMDriver } from "../src/index.ts";

vi.mock("litellm", () => ({
  completion: vi.fn(),
}));

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
  getSecret: (name: string) => secrets[name] ?? "",
  strict: false,
  capabilities: {},
  workflowCapabilities: {},
  resources: {},
  missingSecrets: [],
});

const fakeContext = makeContext();

const makeFetchResponse = (
  content: string,
  usage = { prompt_tokens: 1, completion_tokens: 1, total_tokens: 2 },
) =>
  new Response(
    JSON.stringify({
      choices: [{ message: { content } }],
      usage,
    }),
    { status: 200, headers: { "Content-Type": "application/json" } },
  );

const parseFetchBody = (fetchMock: ReturnType<typeof vi.fn>) => {
  const [, options] = fetchMock.mock.calls[0] as [string, { body: string }];
  return JSON.parse(options.body);
};

it.effect("completes a chat and emits llm.done", () =>
  Effect.gen(function* () {
    const fetchMock = vi.fn().mockResolvedValue(makeFetchResponse("hello"));
    globalThis.fetch = fetchMock as unknown as typeof globalThis.fetch;

    const node = new Node(
      "test-llm",
      "litellm",
      "complete",
      [],
      ["llm.done", "llm.failed"],
      "any",
      { model: "gpt-4", prompt: "say hello" },
    );

    const events = yield* LiteLLMDriver.invoke(node, [], fakeContext);

    expect(events.length).toBe(1);
    expect(events[0].type).toBe("llm.done");
    expect(events[0].payload.content).toBe("hello");
    expect(events[0].payload.model).toBe("gpt-4");
    expect(fetchMock).toHaveBeenCalledTimes(1);
  }),
);

it.effect("llm:complete prepends a system prompt", () =>
  Effect.gen(function* () {
    const fetchMock = vi.fn().mockResolvedValue(makeFetchResponse("ok"));
    globalThis.fetch = fetchMock as unknown as typeof globalThis.fetch;

    const node = new Node("ask", "litellm", "complete", [], ["llm.done"], "any", {
      model: "anthropic/claude-sonnet-4-20250514",
      system: "You are a reviewer.",
      messages: [{ role: "user", content: "Review" }],
    });

    yield* LiteLLMDriver.invoke(node, [], fakeContext);
    const body = parseFetchBody(fetchMock);

    expect(body.messages[0]).toEqual({ role: "system", content: "You are a reviewer." });
    expect(body.messages[1]).toEqual({ role: "user", content: "Review" });
  }),
);

it.effect("llm:complete passes optional parameters", () =>
  Effect.gen(function* () {
    const fetchMock = vi.fn().mockResolvedValue(makeFetchResponse("ok"));
    globalThis.fetch = fetchMock as unknown as typeof globalThis.fetch;

    const node = new Node("ask", "litellm", "complete", [], ["llm.done"], "any", {
      model: "openai/gpt-4o",
      messages: [{ role: "user", content: "Hi" }],
      temperature: 0.1,
      max_tokens: 100,
      timeout: 30,
    });

    yield* LiteLLMDriver.invoke(node, [], fakeContext);
    const body = parseFetchBody(fetchMock);

    expect(body.temperature).toBe(0.1);
    expect(body.max_tokens).toBe(100);
  }),
);

it.effect("llm:complete uses LITELLM_API_KEY from secrets", () =>
  Effect.gen(function* () {
    const fetchMock = vi.fn().mockResolvedValue(makeFetchResponse("ok"));
    globalThis.fetch = fetchMock as unknown as typeof globalThis.fetch;

    const node = new Node("ask", "litellm", "complete", [], ["llm.done"], "any", {
      model: "openai/gpt-4o",
      messages: [{ role: "user", content: "Hi" }],
    });

    const ctx = makeContext({ LITELLM_API_KEY: "sk-123" });
    yield* LiteLLMDriver.invoke(node, [], ctx);

    const [, options] = fetchMock.mock.calls[0] as [string, { headers: Record<string, string> }];
    expect(options.headers["Authorization"]).toBe("Bearer sk-123");
  }),
);

it.effect("llm:complete emits llm.failed on non-ok response", () =>
  Effect.gen(function* () {
    const fetchMock = vi.fn().mockResolvedValue(new Response("error", { status: 500 }));
    globalThis.fetch = fetchMock as unknown as typeof globalThis.fetch;

    const node = new Node("ask", "litellm", "complete", [], ["llm.done", "llm.failed"], "any", {
      model: "openai/gpt-4o",
      prompt: "Hi",
    });

    const events = yield* LiteLLMDriver.invoke(node, [], fakeContext);

    expect(events.length).toBe(1);
    expect(events[0].type).toBe("llm.failed");
    expect(events[0].payload.reason).toContain("status 500");
  }),
);

it.effect("llm:complete emits llm.failed on network error", () =>
  Effect.gen(function* () {
    const fetchMock = vi.fn().mockRejectedValue(new Error("network down"));
    globalThis.fetch = fetchMock as unknown as typeof globalThis.fetch;

    const node = new Node("ask", "litellm", "complete", [], ["llm.done", "llm.failed"], "any", {
      model: "openai/gpt-4o",
      prompt: "Hi",
    });

    const events = yield* LiteLLMDriver.invoke(node, [], fakeContext);

    expect(events.length).toBe(1);
    expect(events[0].type).toBe("llm.failed");
    expect(events[0].payload.reason).toContain("network down");
  }),
);

it.effect("llm:complete fails when prompt and messages are missing", () =>
  Effect.gen(function* () {
    const node = new Node("ask", "litellm", "complete", [], ["llm.done"], "any", {
      model: "openai/gpt-4o",
    });

    const result = yield* Effect.either(LiteLLMDriver.invoke(node, [], fakeContext));

    expect(Either.isLeft(result)).toBe(true);
    if (Either.isLeft(result)) {
      expect(result.left.reason).toContain("missing 'params.messages' or 'params.prompt'");
    }
  }),
);

it.effect("llm:complete_struct returns parsed JSON", () =>
  Effect.gen(function* () {
    const content = '{"ok": true, "items": [1, 2]}';
    const fetchMock = vi.fn().mockResolvedValue(makeFetchResponse(content));
    globalThis.fetch = fetchMock as unknown as typeof globalThis.fetch;

    const node = new Node("classify", "litellm", "complete_struct", [], ["classified"], "any", {
      model: "openai/gpt-4o",
      messages: [{ role: "user", content: "Classify" }],
      schema: { type: "object", properties: { ok: { type: "boolean" } } },
    });

    const events = yield* LiteLLMDriver.invoke(node, [], fakeContext);

    expect(events[0].type).toBe("llm.done");
    expect(events[0].payload.data).toEqual({ ok: true, items: [1, 2] });
    expect(events[0].payload.model).toBe("openai/gpt-4o");

    const body = parseFetchBody(fetchMock);
    expect(body.response_format).toEqual({
      type: "json_schema",
      json_schema: { type: "object", properties: { ok: { type: "boolean" } } },
    });
  }),
);

it.effect("llm:complete_struct falls back to raw content on invalid JSON", () =>
  Effect.gen(function* () {
    const fetchMock = vi.fn().mockResolvedValue(makeFetchResponse("not-json"));
    globalThis.fetch = fetchMock as unknown as typeof globalThis.fetch;

    const node = new Node("classify", "litellm", "complete_struct", [], ["classified"], "any", {
      model: "openai/gpt-4o",
      prompt: "Classify",
    });

    const events = yield* LiteLLMDriver.invoke(node, [], fakeContext);

    expect(events[0].type).toBe("llm.done");
    expect(events[0].payload.data).toEqual({ raw: "not-json" });
  }),
);

it.effect("llm:complete emits llm.failed when content is missing", () =>
  Effect.gen(function* () {
    const fetchMock = vi
      .fn()
      .mockResolvedValue(new Response(JSON.stringify({ choices: [] }), { status: 200 }));
    globalThis.fetch = fetchMock as unknown as typeof globalThis.fetch;

    const node = new Node("ask", "litellm", "complete", [], ["llm.done", "llm.failed"], "any", {
      model: "openai/gpt-4o",
      prompt: "Hi",
    });

    const events = yield* LiteLLMDriver.invoke(node, [], fakeContext);

    expect(events[0].type).toBe("llm.failed");
    expect(events[0].payload.reason).toContain("missing choices[0].message.content");
  }),
);
