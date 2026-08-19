import { Context, Duration, Effect, Either, Layer } from "effect";
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
  private retryCounts = new Map<string, number>();
  private circuitStates = new Map<string, { failures: number; open: boolean; openedAt: number }>();

  constructor(
    private bus: EventBus,
    private registry: DriverRegistry,
    private observer: Observer,
  ) {}

  run(workflow: Workflow, options: RunOptions = {}) {
    return Effect.gen(this, function* (this: RunnerLive, _resume: Adapter) {
      yield* this.bus.reset(workflow.nodes, workflow.config);
      yield* Effect.sync(() => this.resetState());
      const pending: { current: Array<Event> } = { current: [] };
      let ready: Array<readonly [Node, ReadonlyArray<Event>]> = [];
      let failed = false;
      let cancelled = false;
      const dryRun = options.dryRun ?? false;
      const secrets = options.secrets ?? {};
      const requestShutdown = () => {
        cancelled = true;
      };
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
              this.executeNode(node, triggers, pending, dryRun, secrets, workflow, requestShutdown),
            { concurrency: "unbounded" },
          );
          for (const events of emitted) {
            pending.current.push(...events);
          }
          const failedTypes = new Set([
            "node.failed",
            "circuit.open",
            "dlq.event",
            "capability.violation",
          ]);
          if (emitted.some((events) => events.some((event) => failedTypes.has(event.type)))) {
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

  private resetState(): void {
    this.idempotencyCounter = 0;
    this.retryCounts.clear();
    this.circuitStates.clear();
  }

  private executeNode(
    node: Node,
    triggers: ReadonlyArray<Event>,
    pending: { current: Array<Event> },
    dryRun: boolean,
    secrets: Readonly<Record<string, string>>,
    workflow: Workflow,
    requestShutdown: () => void,
  ) {
    return Effect.gen(this, function* (this: RunnerLive, _resume: Adapter) {
      const span = this.observer.onNodeStart(node, triggers);
      const [log, fires] = yield* Effect.all([this.bus.log, this.bus.fireCount(node.id)]);
      const makeCtx: (idempotencyKey: string) => ExecutionContext = (idempotencyKey) =>
        this.makeContext(
          node,
          fires,
          idempotencyKey,
          pending,
          dryRun,
          secrets,
          workflow,
          log,
          requestShutdown,
        );

      if (node.poll) {
        const tick = `${node.id}.tick`;
        const isTick = triggers.some((event) => event.type === tick);
        const interval =
          typeof node.with_.interval === "number"
            ? node.with_.interval
            : typeof node.poll?.interval === "number"
              ? node.poll.interval
              : 0;
        if (isTick && interval > 0) {
          yield* Effect.sleep(Duration.seconds(interval));
        }
      }

      const driverEvents = node.fanIn
        ? ([] as ReadonlyArray<Event>)
        : yield* this.runWithRetry(node, triggers, makeCtx, dryRun);
      const all = this.buildNodeEvents(node, triggers, driverEvents);
      this.observer.onNodeEnd(node, all, null, span);
      return all;
    });
  }

  private buildNodeEvents(
    node: Node,
    triggers: ReadonlyArray<Event>,
    driverEvents: ReadonlyArray<Event>,
  ): ReadonlyArray<Event> {
    const out: Array<Event> = [];
    if (node.fanIn) {
      const fanInEmit = node.fanIn?.emit;
      const emitType =
        typeof fanInEmit === "string" ? fanInEmit : (node.emits[0] ?? `${node.id}.done`);
      out.push(
        new Event(
          emitType,
          {
            count: triggers.length,
            triggers: triggers.map((trigger) => ({
              type: trigger.type,
              source: trigger.source,
              payload: trigger.payload,
            })),
          },
          node.id,
        ),
      );
    } else {
      out.push(...driverEvents);
    }
    if (node.forward.length > 0) {
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
          out.push(new Event(resolved, trigger.payload, node.id));
        }
      }
    }
    if (node.poll) {
      const tick = `${node.id}.tick`;
      if (!out.some((event) => event.type === tick)) {
        out.push(new Event(tick, { poll: true }, node.id));
      }
    }
    return out;
  }

  private runWithRetry(
    node: Node,
    triggers: ReadonlyArray<Event>,
    makeCtx: (idempotencyKey: string) => ExecutionContext,
    dryRun: boolean,
  ) {
    return Effect.gen(this, function* (this: RunnerLive, _resume: Adapter) {
      let now = yield* Effect.clockWith((clock) => clock.currentTimeMillis);
      if (this.isCircuitOpen(node, now)) {
        return [Event.nodeFailed(node.id, "circuit open")];
      }

      const retryCfg = node.retry;
      const maxAttempts = Math.max(1, Number(retryCfg.max_attempts ?? 1));
      let lastError: DriverError | null = null;

      for (let attempt = 1; attempt <= maxAttempts; attempt++) {
        const idempotencyKey = `${node.id}:${++this.idempotencyCounter}`;
        const ctx = makeCtx(idempotencyKey);
        const result = yield* this.runDriver(node, triggers, ctx, dryRun).pipe(Effect.either);
        if (Either.isRight(result)) {
          this.recordSuccess(node);
          this.retryCounts.delete(node.id);
          return result.right;
        }
        lastError = result.left;

        now = yield* Effect.clockWith((clock) => clock.currentTimeMillis);
        if (this.recordFailure(node, now)) {
          const onTripRaw = node.circuitBreaker?.on_trip;
          const onTrip = typeof onTripRaw === "string" ? onTripRaw : "circuit.open";
          return [
            new Event(
              onTrip,
              { node: node.id, failures: this.circuitStates.get(node.id)?.failures ?? 0 },
              node.id,
            ),
            Event.nodeFailed(node.id, `circuit breaker tripped for '${node.id}'`),
          ];
        }
        this.retryCounts.set(node.id, attempt);

        if (attempt < maxAttempts) {
          const baseDelay = Number(retryCfg.delay ?? 1);
          const backoffRaw = retryCfg.backoff;
          const backoff = typeof backoffRaw === "string" ? backoffRaw : "fixed";
          let delay = baseDelay;
          if (backoff === "exponential") {
            delay = baseDelay * Math.pow(2, attempt - 1);
          } else if (backoff === "linear") {
            delay = baseDelay * attempt;
          }
          yield* Effect.sleep(Duration.seconds(delay));
        }
      }

      return [
        Event.nodeFailed(node.id, lastError?.reason ?? "unknown error"),
        new Event(
          "dlq.event",
          {
            node: node.id,
            error: lastError?.reason ?? "unknown error",
            event: "node.failed",
          },
          "__runner__",
        ),
      ];
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

  private isCircuitOpen(node: Node, now: number): boolean {
    const cb = this.circuitStates.get(node.id);
    if (!cb || !cb.open) return false;
    const resetAfter = Number(node.circuitBreaker?.reset_after ?? 60) * 1000;
    if (now - cb.openedAt > resetAfter) {
      cb.open = false;
      cb.failures = 0;
      return false;
    }
    return true;
  }

  private recordFailure(node: Node, now: number): boolean {
    if (!node.circuitBreaker || Object.keys(node.circuitBreaker).length === 0) {
      return false;
    }
    const cb = this.circuitStates.get(node.id) ?? { failures: 0, open: false, openedAt: 0 };
    cb.failures += 1;
    const threshold = Number(node.circuitBreaker?.threshold ?? 5);
    if (!cb.open && cb.failures >= threshold) {
      cb.open = true;
      cb.openedAt = now;
      this.circuitStates.set(node.id, cb);
      return true;
    }
    this.circuitStates.set(node.id, cb);
    return false;
  }

  private recordSuccess(node: Node): void {
    const cb = this.circuitStates.get(node.id);
    if (cb) {
      cb.failures = 0;
      cb.open = false;
    }
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
    requestShutdown: () => void,
  ) {
    const state = new SimpleStateStore();
    const capabilities = { ...workflow.capabilities, ...node.capabilities };
    return {
      config: workflow.config,
      secrets,
      log,
      emit: (event: Event) => {
        pending.current.push(event);
      },
      inject: (nodes: ReadonlyArray<Record<string, unknown>>) => {
        this.bus.inject(nodes);
      },
      dryRun,
      idempotencyKey,
      traceparent: "",
      state,
      fires,
      requestShutdown,
      getSecret: (key: string, default_ = "") => secrets[key] ?? default_,
      resources: node.resources,
      capabilities,
      workflowCapabilities: workflow.capabilities,
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
