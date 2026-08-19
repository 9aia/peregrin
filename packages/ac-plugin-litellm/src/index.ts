import { Context, Either, Effect, Layer } from "effect";
import { Driver, DriverError, Event, type ExecutionContext, type Node } from "ac-core";
import { resolveValueForNode } from "ac-template-engine";

const defaultEndpoint = "http://127.0.0.1:4000/v1/chat/completions";

export interface LiteLLMRequest {
  readonly model: string;
  readonly provider?: string | undefined;
  readonly prompt: string;
  readonly endpoint?: string | undefined;
  readonly apiKey?: string | undefined;
  readonly system?: string | undefined;
}

export interface LiteLLM {
  readonly complete: (request: LiteLLMRequest) => Effect.Effect<string, DriverError>;
}

export const LiteLLM = Context.GenericTag<LiteLLM>("ac/LiteLLM");

interface Usage {
  readonly prompt_tokens: number;
  readonly completion_tokens: number;
  readonly total_tokens: number;
}

interface ChatResult {
  readonly content: string;
  readonly usage: Usage;
  readonly raw: unknown;
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
  value !== null && typeof value === "object" && !Array.isArray(value);

const makeUsage = (raw: unknown): Usage => {
  const record = isRecord(raw) ? raw : {};
  const get = (key: string) => {
    const value = record[key];
    return typeof value === "number" ? value : 0;
  };
  return {
    prompt_tokens: get("prompt_tokens"),
    completion_tokens: get("completion_tokens"),
    total_tokens: get("total_tokens"),
  };
};

const extractContent = (data: unknown): string | null => {
  if (!isRecord(data)) return null;
  const choices = data.choices;
  if (!Array.isArray(choices) || choices.length === 0) return null;
  const first = choices[0];
  if (!isRecord(first)) return null;
  const message = first.message;
  if (!isRecord(message)) return null;
  const content = message.content;
  return typeof content === "string" ? content : null;
};

interface Message {
  readonly role: string;
  readonly content: string;
}

interface ChatCompletionArgs {
  readonly model: string;
  readonly messages: ReadonlyArray<Message>;
  readonly endpoint?: string | undefined;
  readonly apiKey?: string | undefined;
  readonly temperature?: number | undefined;
  readonly maxTokens?: number | undefined;
  readonly timeout?: number | undefined;
  readonly responseFormat?: unknown;
}

const chatCompletion = (args: ChatCompletionArgs): Effect.Effect<ChatResult, DriverError> =>
  Effect.gen(function* () {
    const endpoint = args.endpoint && args.endpoint.length > 0 ? args.endpoint : defaultEndpoint;

    const headers: Record<string, string> = { "Content-Type": "application/json" };
    if (args.apiKey && args.apiKey.length > 0) {
      headers["Authorization"] = `Bearer ${args.apiKey}`;
    }

    const body: Record<string, unknown> = {
      model: args.model,
      messages: args.messages,
    };
    if (typeof args.temperature === "number") {
      body.temperature = args.temperature;
    }
    if (typeof args.maxTokens === "number") {
      body.max_tokens = args.maxTokens;
    }
    if (args.responseFormat !== undefined) {
      body.response_format = args.responseFormat;
    }

    const timeout = typeof args.timeout === "number" && args.timeout > 0 ? args.timeout : 120;
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), timeout * 1000);

    const request = Effect.tryPromise({
      try: () =>
        fetch(endpoint, {
          method: "POST",
          headers,
          body: JSON.stringify(body),
          signal: controller.signal,
        }),
      catch: (error) =>
        new DriverError({
          reason: `litellm request failed for model '${args.model}': ${String(error)}`,
        }),
    }).pipe(Effect.ensuring(Effect.sync(() => clearTimeout(timeoutId))));

    const response = yield* request;

    if (!response.ok) {
      return yield* Effect.fail(
        new DriverError({
          reason: `litellm request failed with status ${response.status}`,
        }),
      );
    }

    const raw = yield* Effect.tryPromise({
      try: () => response.json() as Promise<unknown>,
      catch: (error) =>
        new DriverError({
          reason: `failed to parse litellm response: ${String(error)}`,
        }),
    });

    const content = extractContent(raw);
    if (content === null) {
      return yield* Effect.fail(
        new DriverError({
          reason: "litellm response missing choices[0].message.content",
        }),
      );
    }

    return { content, usage: makeUsage(isRecord(raw) ? raw.usage : undefined), raw };
  });

const complete = (request: LiteLLMRequest): Effect.Effect<string, DriverError> =>
  Effect.gen(function* () {
    const model =
      request.provider && request.provider.length > 0
        ? `${request.provider}/${request.model}`
        : request.model;

    const messages: Array<Message> = [];
    if (request.system && request.system.length > 0) {
      messages.push({ role: "system", content: request.system });
    }
    messages.push({ role: "user", content: request.prompt });

    const result = yield* chatCompletion({
      model,
      messages,
      endpoint: request.endpoint,
      apiKey: request.apiKey,
    });

    return result.content;
  });

const makeMessages = (with_: Record<string, unknown>): ReadonlyArray<Message> | null => {
  const system =
    typeof with_.system === "string" && with_.system.length > 0 ? with_.system : undefined;
  const rawMessages = with_.messages;
  const prompt = with_.prompt;

  const messages: Array<Message> = [];
  if (system !== undefined) {
    messages.push({ role: "system", content: system });
  }

  if (Array.isArray(rawMessages)) {
    for (const raw of rawMessages) {
      if (isRecord(raw)) {
        const role = raw.role;
        const content = raw.content;
        if (typeof role === "string" && typeof content === "string") {
          messages.push({ role, content });
        }
      }
    }
    return messages.length > 0 ? messages : null;
  }

  if (typeof prompt === "string" && prompt.length > 0) {
    messages.push({ role: "user", content: prompt });
    return messages;
  }

  return null;
};

const resolveApiKey = (
  with_: Record<string, unknown>,
  ctx: ExecutionContext,
): string | undefined => {
  const fromWith =
    typeof with_.api_key === "string" && with_.api_key.length > 0 ? with_.api_key : undefined;
  return fromWith ?? ctx.secrets["LITELLM_API_KEY"];
};

export const LiteLLMDriver: Driver = {
  name: "litellm",
  invoke: (node: Node, triggers: ReadonlyArray<Event>, ctx: ExecutionContext) =>
    Effect.gen(function* () {
      const resolved = yield* resolveValueForNode(node.with_, triggers, ctx, node).pipe(
        Effect.mapError((error) => new DriverError({ reason: String(error) })),
      );

      const with_ = resolved as Record<string, unknown>;
      const model =
        typeof with_.model === "string" && with_.model.length > 0 ? with_.model : "openai/gpt-4o";

      const messages = makeMessages(with_);
      if (messages === null) {
        return yield* Effect.fail(
          new DriverError({
            reason: `litellm node '${node.id}' missing 'with.messages' or 'with.prompt'`,
          }),
        );
      }

      const responseFormat =
        node.handler === "complete_struct"
          ? isRecord(with_.schema)
            ? { type: "json_schema", json_schema: with_.schema }
            : { type: "json_object" }
          : undefined;

      const result = yield* chatCompletion({
        model,
        messages,
        endpoint:
          typeof with_.endpoint === "string" && with_.endpoint.length > 0
            ? with_.endpoint
            : undefined,
        apiKey: resolveApiKey(with_, ctx),
        temperature: typeof with_.temperature === "number" ? with_.temperature : undefined,
        maxTokens: typeof with_.max_tokens === "number" ? with_.max_tokens : undefined,
        timeout: typeof with_.timeout === "number" && with_.timeout > 0 ? with_.timeout : undefined,
        responseFormat,
      }).pipe(Effect.either);

      if (Either.isLeft(result)) {
        return [new Event("llm.failed", { reason: result.left.reason, model }, node.id)];
      }

      const { content, usage, raw } = result.right;
      const payload: Record<string, unknown> = {
        content,
        model,
        usage,
        raw_response: raw,
      };

      if (node.handler === "complete_struct") {
        const data = yield* Effect.try({
          try: () => JSON.parse(content) as unknown,
          catch: () => "parse",
        }).pipe(Effect.catchAll(() => Effect.succeed({ raw: content })));
        payload.data = data;
      }

      return [new Event("llm.done", payload, node.id)];
    }),
};

export const LiteLLMDriverLive = Layer.succeed(Driver, LiteLLMDriver);

export const Live = Layer.succeed(LiteLLM, {
  complete: (request) => complete(request),
});
