import { Context, Effect, Layer } from "effect";
import type { Adapter } from "effect/Effect";
import { Event, Node, Workflow, DriverError, Observer } from "ac-core";
import type { ExecutionContext, StateStore } from "ac-core";
import { captureWildcards, EventBus } from "./bus.js";
import { DriverRegistry } from "./registry.js";

function resolveForward(fwdType: string, captures: Record<string, string>): string {
  const parts = fwdType.split(".");
  const out: Array<string> = [];
  let capIdx = 1;
  for (const part of parts) {
    if (part === "*" || part === "**") {
      out.push(captures[String(capIdx)] ?? "*");
      capIdx++;
    } else {
      out.push(part);
    }
  }
  return out.join(".");
}

class SimpleStateStore implements StateStore {
  private data = new Map<string, unknown>();

  get(key: string) {
    return this.data.get(key);
  }

  set(key: string, value: unknown) {
    this.data.set(key, value);
  }

  toRecord() {
    return Object.fromEntries(this.data.entries()) as Readonly<Record<string, unknown>>;
  }
}

export interface RunOptions {
  readonly dryRun?: boolean;
  readonly secrets?: Readonly<Record<string, string>>;
}

export interface Runner {
  readonly run: (
    workflow: Workflow,
    options?: RunOptions,
  ) => Effect.Effect<ReadonlyArray<Event>, DriverError, never>;
}

export const Runner = Context.GenericTag<Runner>("ac/Runner");

class RunnerLive implements Runner {
  private idempotencyCounter = 0;

  constructor(
    private bus: EventBus,
    private registry: DriverRegistry,
    private observer: Observer,
  ) {}

  run(workflow: Workflow, options: RunOptions = {}) {
    return Effect.gen(this, function* (this: RunnerLive, _resume: Adapter) {
      yield* this.bus.reset(workflow.nodes, workflow.config);
      const pending: { current: Array<Event> } = { current: [] };
      let ready: Array<readonly [Node, ReadonlyArray<Event>]> = [];
      let failed = false;
      let cancelled = false;
      const dryRun = options.dryRun ?? false;
      const secrets = options.secrets ?? {};
      yield* Effect.sync(() => this.observer.onWorkflowStart(workflow));
      const start = yield* this.bus.publish(Event.start());
      ready = Array.from(start.ready);
      pending.current.push(...start.sideEvents);
      while ((pending.current.length > 0 || ready.length > 0) && !cancelled) {
        if (ready.length > 0) {
          const toRun = ready;
          ready = [];
          const emitted = yield* Effect.forEach(
            toRun,
            ([node, triggers]) =>
              this.executeNode(node, triggers, pending, dryRun, secrets, workflow),
            { concurrency: "unbounded" },
          );
          for (const events of emitted) {
            pending.current.push(...events);
          }
          if (emitted.some((events) => events.some((event) => event.type === "node.failed"))) {
            failed = true;
          }
        }
        if (cancelled || pending.current.length === 0) continue;
        const batch = pending.current;
        pending.current = [];
        for (const event of batch) {
          if (event.type === "workflow.cancel") {
            cancelled = true;
            break;
          }
          if (workflow.stopOn.includes(event.type)) {
            cancelled = true;
            break;
          }
          const result = yield* this.bus.publish(event);
          pending.current.push(...result.sideEvents);
          ready = [...ready, ...result.ready];
        }
      }
      if (cancelled) {
        yield* this.bus.publish(
          new Event("workflow.cancelled", { reason: "cancelled" }, "__runner__"),
        );
      } else if (failed) {
        yield* this.bus.publish(Event.workflowFailed("", "node failure"));
      } else {
        yield* this.bus.publish(Event.done());
      }
      const log = yield* this.bus.log;
      yield* Effect.sync(() => this.observer.onWorkflowEnd(log.length, cancelled, false, failed));
      return log;
    });
  }

  private executeNode(
    node: Node,
    triggers: ReadonlyArray<Event>,
    pending: { current: Array<Event> },
    dryRun: boolean,
    secrets: Readonly<Record<string, string>>,
    workflow: Workflow,
  ) {
    return Effect.gen(this, function* (this: RunnerLive, _resume: Adapter) {
      const span = this.observer.onNodeStart(node, triggers);
      const [log, fires, idempotencyKey] = yield* Effect.all([
        this.bus.log,
        this.bus.fireCount(node.id),
        Effect.sync(() => `${node.id}:${++this.idempotencyCounter}`),
      ]);
      const ctx = this.makeContext(
        node,
        fires,
        idempotencyKey,
        pending,
        dryRun,
        secrets,
        workflow,
        log,
      );
      const events = yield* this.runDriver(node, triggers, ctx, dryRun).pipe(
        Effect.match({
          onFailure: (error) => {
            this.observer.onNodeEnd(node, [], new Error(error.reason), span);
            return [Event.nodeFailed(node.id, error.reason)];
          },
          onSuccess: (driverEvents) => {
            const forwarded: Array<Event> = [];
            if (node.forward) {
              for (const trigger of triggers) {
                for (const fwd of node.forward) {
                  let resolved = fwd;
                  for (const onPattern of node.on) {
                    const caps = captureWildcards(onPattern, trigger.type);
                    if (caps) {
                      resolved = resolveForward(fwd, caps);
                      break;
                    }
                  }
                  forwarded.push(new Event(resolved, trigger.payload, node.id));
                }
              }
            }
            const all = [...driverEvents, ...forwarded];
            this.observer.onNodeEnd(node, all, null, span);
            return all;
          },
        }),
      );
      return events;
    });
  }

  private runDriver(
    node: Node,
    triggers: ReadonlyArray<Event>,
    ctx: ExecutionContext,
    dryRun: boolean,
  ) {
    return Effect.gen(this, function* (this: RunnerLive, _resume: Adapter) {
      if (dryRun) {
        return [new Event(node.emits[0] ?? `${node.id}.done`, { dry_run: true }, node.id)];
      }
      if (!node.driver) {
        return [];
      }
      const driver = yield* this.registry.resolve(node.driver);
      return yield* driver.invoke(node, triggers, ctx);
    });
  }

  private makeContext(
    node: Node,
    fires: number,
    idempotencyKey: string,
    pending: { current: Array<Event> },
    dryRun: boolean,
    secrets: Readonly<Record<string, string>>,
    workflow: Workflow,
    log: ReadonlyArray<Event>,
  ): ExecutionContext {
    const state = new SimpleStateStore();
    return {
      config: workflow.config,
      secrets,
      log,
      emit: (event) => {
        pending.current.push(event);
      },
      inject: () => {},
      dryRun,
      idempotencyKey,
      traceparent: "",
      state,
      fires,
      requestShutdown: () => {},
      getSecret: (key, default_ = "") => secrets[key] ?? default_,
    };
  }
}

export const RunnerLiveLayer = Layer.effect(
  Runner,
  Effect.gen(function* () {
    const bus = yield* EventBus;
    const registry = yield* DriverRegistry;
    const observer = yield* Observer;
    return new RunnerLive(bus, registry, observer);
  }),
);
