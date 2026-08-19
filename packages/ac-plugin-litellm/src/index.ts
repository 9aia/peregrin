import { Context, Effect, Layer } from "effect";
import { DriverError } from "ac-core";

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

const defaultEndpoint = "http://127.0.0.1:4000/v1/chat/completions";

const extractContent = (data: unknown): string | null => {
  if (data === null || typeof data !== "object") return null;
  const record = data as Record<string, unknown>;
  const choices = record.choices;
  if (!Array.isArray(choices) || choices.length === 0) return null;
  const first = choices[0];
  if (first === null || typeof first !== "object") return null;
  const message = (first as Record<string, unknown>).message;
  if (message === null || typeof message !== "object") return null;
  const content = (message as Record<string, unknown>).content;
  return typeof content === "string" ? content : null;
};

const complete = (request: LiteLLMRequest): Effect.Effect<string, DriverError> =>
  Effect.gen(function* () {
    const model =
      request.provider && request.provider.length > 0
        ? `${request.provider}/${request.model}`
        : request.model;

    const headers: Record<string, string> = { "Content-Type": "application/json" };
    if (request.apiKey && request.apiKey.length > 0) {
      headers["Authorization"] = `Bearer ${request.apiKey}`;
    }

    const messages: Array<Record<string, string>> = [];
    if (request.system && request.system.length > 0) {
      messages.push({ role: "system", content: request.system });
    }
    messages.push({ role: "user", content: request.prompt });

    const response = yield* Effect.tryPromise({
      try: () =>
        fetch(request.endpoint ?? defaultEndpoint, {
          method: "POST",
          headers,
          body: JSON.stringify({ model, messages }),
        }),
      catch: (error) =>
        new DriverError({
          reason: `litellm request failed for model '${model}': ${String(error)}`,
        }),
    });

    if (!response.ok) {
      return yield* Effect.fail(
        new DriverError({
          reason: `litellm request failed with status ${response.status}`,
        }),
      );
    }

    const data = yield* Effect.tryPromise({
      try: () => response.json() as Promise<unknown>,
      catch: (error) =>
        new DriverError({
          reason: `failed to parse litellm response: ${String(error)}`,
        }),
    });

    const content = extractContent(data);
    if (content === null) {
      return yield* Effect.fail(
        new DriverError({
          reason: "litellm response missing choices[0].message.content",
        }),
      );
    }

    return content;
  });

export const Live = Layer.succeed(LiteLLM, {
  complete: (request) => complete(request),
});
