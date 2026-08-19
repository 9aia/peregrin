import fs from "node:fs";
import path from "node:path";
import { createInterface } from "node:readline/promises";
import { Effect, Layer } from "effect";
import { Driver, DriverError, Event, type ExecutionContext, type Node } from "ac-core";
import { resolveValueForNode } from "ac-template-engine";

export const HumanDriver: Driver = {
  name: "human",
  invoke: (node: Node, triggers: ReadonlyArray<Event>, ctx: ExecutionContext) =>
    Effect.gen(function* () {
      const resolved = yield* resolveValueForNode(node.with_, triggers, ctx, node).pipe(
        Effect.mapError((error) => new DriverError({ reason: String(error) })),
      );

      const with_ = resolved as Record<string, unknown>;
      const prompt = with_.prompt;
      if (typeof prompt !== "string" || prompt.length === 0) {
        return yield* Effect.fail(
          new DriverError({ reason: `human node '${node.id}' missing 'with.prompt'` }),
        );
      }

      let finalPrompt = prompt;

      const promptFile = with_.prompt_file;
      if (typeof promptFile === "string" && promptFile.length > 0) {
        const filePath = path.isAbsolute(promptFile)
          ? promptFile
          : path.join(process.cwd(), promptFile);
        const fileContents = yield* Effect.try({
          try: () => fs.readFileSync(filePath, "utf-8"),
          catch: (error) => new DriverError({ reason: String(error) }),
        });
        finalPrompt = fileContents.length > 0 ? fileContents + "\n" + prompt : prompt;
      }

      const expectValue = typeof with_.expect === "string" ? with_.expect : undefined;

      const rawTimeout = with_.timeout;
      const timeoutSeconds = typeof rawTimeout === "number" ? rawTimeout : Number(rawTimeout ?? 0);
      const timeoutMs = timeoutSeconds > 0 ? timeoutSeconds * 1000 : 0;

      const emitOk = typeof with_.emit_ok === "string" ? with_.emit_ok : undefined;
      const emitReject = typeof with_.emit_reject === "string" ? with_.emit_reject : undefined;

      if (ctx.dryRun) {
        const eventType = emitOk ?? node.emits[0] ?? `${node.id}.ok`;
        return [
          new Event(eventType, { response: "(dry-run)", ok: true, timed_out: false }, node.id),
        ];
      }

      const rl = createInterface({
        input: process.stdin,
        output: process.stderr,
      });

      const controller = new AbortController();
      const question = Effect.tryPromise({
        try: () => rl.question(finalPrompt, { signal: controller.signal }),
        catch: (error) => new DriverError({ reason: String(error) }),
      });

      let timedOut = false;
      const timeout =
        timeoutMs > 0
          ? Effect.sleep(timeoutMs).pipe(
              Effect.tap(() =>
                Effect.try({
                  try: () => {
                    timedOut = true;
                    controller.abort();
                  },
                  catch: (error) => new DriverError({ reason: String(error) }),
                }),
              ),
              Effect.map(() => ""),
            )
          : Effect.never;

      const rawResponse =
        timeoutMs > 0
          ? yield* Effect.race(question, timeout).pipe(
              Effect.ensuring(
                Effect.sync(() => {
                  try {
                    rl.close();
                  } catch {
                    // no-op
                  }
                }),
              ),
            )
          : yield* question.pipe(
              Effect.ensuring(
                Effect.sync(() => {
                  try {
                    rl.close();
                  } catch {
                    // no-op
                  }
                }),
              ),
            );

      const response = rawResponse.trim();

      let ok: boolean;
      if (timedOut) {
        ok = false;
      } else if (expectValue !== undefined) {
        ok = response === expectValue;
      } else {
        ok = response !== "";
      }

      const eventType = ok
        ? (emitOk ?? node.emits[0] ?? `${node.id}.ok`)
        : (emitReject ??
          node.emits[1] ??
          node.emits[node.emits.length - 1] ??
          `${node.id}.rejected`);

      return [new Event(eventType, { response, ok, timed_out: timedOut }, node.id)];
    }),
};

export const HumanDriverLive = Layer.succeed(Driver, HumanDriver);
