import { it, expect } from "@effect/vitest";
import { vi } from "vite-plus/test";
import { Effect } from "effect";
import { Node } from "ac-core";
import type { ExecutionContext } from "ac-core";
import { LiteLLMDriver } from "../src/index.ts";

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

it.effect("completes a chat and emits llm.done", () =>
  Effect.gen(function* () {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          choices: [{ message: { content: "hello" } }],
          usage: { prompt_tokens: 1, completion_tokens: 1, total_tokens: 2 },
        }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );
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
