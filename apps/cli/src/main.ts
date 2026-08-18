import { Args, Command } from "@effect/cli";
import { NodeContext, NodeRuntime } from "@effect/platform-node";
import { Console, Effect } from "effect";
import { loadWorkflow } from "ac-loader-yaml";
import { Runner } from "ac-engine";
import {
  RunnerLiveLayer,
  EventBusLiveLayer,
  DefaultDriverRegistryLive,
  NoopObserverLayer,
  MemoryStoreLive,
} from "ac-cli-defaults";

const workflowArg = Args.text({ name: "workflow" }).pipe(
  Args.withDescription("Path to the workflow to run"),
);

const run = Command.make("run", { workflow: workflowArg }, ({ workflow }) =>
  Effect.gen(function* () {
    const runner = yield* Runner;
    const wf = yield* loadWorkflow(workflow);
    const events = yield* runner.run(wf);
    for (const event of events) {
      yield* Console.log(`${event.ts} ${event.source} ${event.type}`);
    }
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

program(process.argv).pipe(
  Effect.provide(RunnerLiveLayer),
  Effect.provide(EventBusLiveLayer),
  Effect.provide(DefaultDriverRegistryLive),
  Effect.provide(NoopObserverLayer),
  Effect.provide(MemoryStoreLive),
  Effect.provide(NodeContext.layer),
  NodeRuntime.runMain,
);
