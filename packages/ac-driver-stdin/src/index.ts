import { createInterface } from "node:readline/promises";
import { Effect, Layer } from "effect";
import { Driver, DriverError, Event, type ExecutionContext, type Node } from "handl-core";
import { resolveValueForNode } from "handl-template-engine";

const readLines = (): Promise<ReadonlyArray<string>> =>
  new Promise((resolve) => {
    const lines: string[] = [];
    const rl = createInterface({
      input: process.stdin,
      output: process.stderr,
    });
    rl.on("line", (line) => {
      const stripped = line.replace(/\r+$/, "");
      if (stripped.length > 0) {
        lines.push(stripped);
      }
    });
    rl.once("close", () => {
      resolve(lines);
    });
  });

const parseLine = (
  line: string,
  defaultEventType: string,
  parseJson: boolean,
): Effect.Effect<Event, never> =>
  Effect.gen(function* () {
    if (!parseJson) {
      return new Event(defaultEventType, { line }, "");
    }

    const parsed = yield* Effect.try(() => JSON.parse(line) as unknown).pipe(
      Effect.orElseSucceed(() => ({ line })),
    );

    if (parsed !== null && typeof parsed === "object" && !Array.isArray(parsed)) {
      const record = parsed as Record<string, unknown>;
      if (typeof record.type === "string") {
        const { type, ...rest } = record;
        return new Event(type, rest, "");
      }
      return new Event(defaultEventType, record, "");
    }

    return new Event(defaultEventType, { line }, "");
  });

export const StdinDriver: Driver = {
  name: "stdin",
  invoke: (node: Node, triggers: ReadonlyArray<Event>, ctx: ExecutionContext) =>
    Effect.gen(function* () {
      const resolved = yield* resolveValueForNode(node.params, triggers, ctx, node).pipe(
        Effect.mapError((error) => new DriverError({ reason: String(error) })),
      );

      const params = resolved as Record<string, unknown>;
      const rawEventType = params.event_type;
      const eventType =
        typeof rawEventType === "string" && rawEventType.length > 0
          ? rawEventType
          : (node.emits[0] ?? `${node.id}.line`);
      const parseJson = params.parse_json === true || params.parse_json === "true";
      const doneEvent = `${node.id}.done`;

      if (ctx.dryRun) {
        return [new Event(doneEvent, { dry_run: true }, node.id)];
      }

      const listening = new Event(
        `${node.id}.listening`,
        { event_type: eventType, parse_json: parseJson },
        node.id,
      );

      const lines = yield* Effect.tryPromise({
        try: () => readLines(),
        catch: (error) => new DriverError({ reason: String(error) }),
      });

      const events: Event[] = [listening];
      for (const line of lines) {
        const event = yield* parseLine(line, eventType, parseJson);
        events.push(new Event(event.type, event.payload, node.id));
      }
      events.push(new Event(doneEvent, { lines_read: true }, node.id));

      return events;
    }),
};

export const StdinDriverLive = Layer.succeed(Driver, StdinDriver);
