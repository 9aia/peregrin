import { Args, Command, Options } from "@effect/cli";
import { NodeContext, NodeRuntime } from "@effect/platform-node";
import { Console, Effect, Option } from "effect";
import { Event } from "ac-core";
import { loadWorkflow, LoaderError } from "ac-loader-yaml";
import { Runner, type RunOptions } from "ac-engine";
import {
  makeDefaultsLive,
  MetricsObserverLive,
  NoopObserverLayer,
  OtelObserverLive,
} from "ac-cli-defaults";
import fs from "node:fs";

const workflowArg = Args.text({ name: "workflow" }).pipe(
  Args.withDescription("Path to the workflow to run"),
);

const replayOption = Options.optional(
  Options.text("replay").pipe(Options.withDescription("Path to a JSON/JSONL event log to replay")),
);

const otelOption = Options.boolean("otel", { ifPresent: true }).pipe(
  Options.withDescription("Enable OpenTelemetry observer"),
);

const metricsOption = Options.boolean("metrics", { ifPresent: true }).pipe(
  Options.withDescription("Emit runtime metrics"),
);

const loadReplayEvents = (
  filePath: string,
): Effect.Effect<ReadonlyArray<Event>, LoaderError, never> =>
  Effect.gen(function* () {
    const content = yield* Effect.try(() => fs.readFileSync(filePath, "utf8")).pipe(
      Effect.mapError(
        (error) =>
          new LoaderError({ reason: `Failed to read replay log: ${filePath}`, cause: error }),
      ),
    );
    const raw = yield* Effect.try(() => {
      const trimmed = content.trim();
      if (trimmed.startsWith("[")) {
        return JSON.parse(trimmed) as Array<Record<string, unknown>>;
      }
      return trimmed
        .split("\n")
        .filter((line) => line.trim() !== "")
        .map((line) => JSON.parse(line) as Record<string, unknown>);
    }).pipe(
      Effect.mapError(
        (error) => new LoaderError({ reason: "Failed to parse replay log", cause: error }),
      ),
    );
    return raw.map(
      (record) =>
        new Event(
          typeof record.type === "string" ? record.type : "",
          (record.payload ?? {}) as Record<string, unknown>,
          typeof record.source === "string" ? record.source : "",
          typeof record.id === "string" ? record.id : "",
          typeof record.ts === "number" ? record.ts : 0,
        ),
    );
  });

const selectObserver = (flags: { otel: boolean; metrics: boolean }) => {
  if (flags.metrics) return MetricsObserverLive;
  if (flags.otel) return OtelObserverLive;
  return NoopObserverLayer;
};

const run = Command.make(
  "run",
  {
    workflow: workflowArg,
    replay: replayOption,
    otel: otelOption,
    metrics: metricsOption,
  },
  ({ workflow, replay, otel, metrics }) =>
    Effect.gen(function* () {
      const wf = yield* loadWorkflow(workflow);
      let replayEvents: ReadonlyArray<Event> = [];
      if (Option.isSome(replay)) {
        replayEvents = yield* loadReplayEvents(replay.value);
      }
      const runOptions: RunOptions = replayEvents.length > 0 ? { replayEvents } : {};
      const observer = selectObserver({ otel, metrics });
      const runWorkflow = Effect.gen(function* () {
        const runner = yield* Runner;
        const events = yield* runner.run(wf, runOptions);
        for (const event of events) {
          yield* Console.log(`${event.ts} ${event.source} ${event.type}`);
        }
      });
      yield* runWorkflow.pipe(Effect.provide(makeDefaultsLive(observer)));
    }),
).pipe(Command.withDescription("Run a workflow"));

const version = Command.make("version", {}, () => Console.log("0.0.0")).pipe(
  Command.withDescription("Show the CLI version"),
);

const ac = Command.make("ac").pipe(Command.withSubcommands([run, version]));

const program = Command.run(ac, {
  name: "ac",
  version: "0.0.0",
});

NodeRuntime.runMain(Effect.provide(program(process.argv), NodeContext.layer));
