import { Context, Duration, Effect, Either, Layer } from "effect";
import type { Adapter } from "effect/Effect";
import {
  Driver,
  Event,
  EventSchema,
  Node,
  Workflow,
  DriverError,
  DeadLetterQueue,
  Observer,
  StateStore,
  TemplateError,
} from "ac-core";
import type { ExecutionContext } from "ac-core";
import { captureWildcards, EventBus, matches } from "./bus.js";
import { DriverRegistry } from "./registry.js";
import { buildValidator, CapabilityChecker, ResourceGate, validateObject } from "ac-safety";
import { resolveTemplate, resolveValueForNode } from "ac-template-engine";

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

function resolveOutputSchemas(
  driver: ReadonlyArray<EventSchema>,
  node: ReadonlyArray<EventSchema>,
): ReadonlyArray<EventSchema> {
  const byType = new Map<string, EventSchema>();
  for (const s of driver) byType.set(s.type, s);
  for (const s of node) byType.set(s.type, s);
  return Array.from(byType.values());
}

function matchingOutputSchema(
  schemas: ReadonlyArray<EventSchema>,
  eventType: string,
): EventSchema | null {
  for (const s of schemas) {
    if (s.type === eventType) return s;
  }
  for (const s of schemas) {
    if (s.type.includes("*") && matches(s.type, eventType)) return s;
  }
  return null;
}

class ScopedStateStore {
  private data = new Map<string, unknown>();

  constructor(initial: Record<string, unknown> = {}) {
    for (const [k, v] of Object.entries(initial)) {
      this.data.set(k, v);
    }
  }

  get(key: string): unknown {
    return this.data.get(key);
  }

  set(key: string, value: unknown): void {
    this.data.set(key, value);
  }

  toRecord(): Readonly<Record<string, unknown>> {
    return Object.fromEntries(this.data.entries()) as Readonly<Record<string, unknown>>;
  }

  clear(): void {
    this.data.clear();
  }

  close(): void {}
}

const noOpDeadLetterQueue: DeadLetterQueue = {
  enqueue: () => Effect.sync(() => undefined),
  all: () => Effect.succeed([]),
  close: () => Effect.sync(() => undefined),
};

export interface RunOptions {
  readonly dryRun?: boolean;
  readonly secrets?: Readonly<Record<string, string>>;
  readonly replayEvents?: ReadonlyArray<Event>;
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
    private resourceGate: ResourceGate,
    private dlq: DeadLetterQueue,
    private stateStore: StateStore,
  ) {}

  run(workflow: Workflow, options: RunOptions = {}) {
    return Effect.gen(this, function* (this: RunnerLive, _resume: Adapter) {
      if (workflow.strict) {
        for (const eventSchema of workflow.events) {
          if (eventSchema.compiledSchema() === null) {
            return yield* Effect.fail(
              new DriverError({
                reason: `strict mode requires a payload schema for event '${eventSchema.type}'`,
              }),
            );
          }
        }
        for (const node of workflow.nodes) {
          if (node.on.length === 0) {
            return yield* Effect.fail(
              new DriverError({
                reason: `strict mode: node '${node.id}' must declare 'on'`,
              }),
            );
          }
          if (node.emits.length === 0) {
            return yield* Effect.fail(
              new DriverError({
                reason: `strict mode: node '${node.id}' must declare 'emits'`,
              }),
            );
          }
        }
      }

      const validator = buildValidator(workflow.events);
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
      const hasReplay = options.replayEvents && options.replayEvents.length > 0;
      const start = hasReplay
        ? yield* this.bus
            .seed(options.replayEvents as ReadonlyArray<Event>)
            .pipe(Effect.flatMap(() => this.bus.checkReady()))
        : yield* this.bus.publish(Event.start());
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
          const failedTypes = new Set(["node.failed", "circuit.open", "workflow.failed"]);
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
          if (validator !== null) {
            const errors = validator.validate(event);
            if (errors.length > 0) {
              pending.current.push(
                Event.eventInvalid(event.type, event.source, errors, event.payload),
              );
              if (workflow.strict) {
                pending.current.push(
                  Event.nodeFailed(event.source, `payload validation failed: ${errors.join("; ")}`),
                );
              }
              continue;
            }
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

  private resourceWeight(node: Node): number {
    const raw = node.resources?.["weight"];
    const n = typeof raw === "number" ? raw : Number(raw);
    if (Number.isNaN(n) || n < 1) return 1;
    return n;
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

      const run = Effect.gen(this, function* (this: RunnerLive, _resume: Adapter) {
        const preKey = `${node.id}:pre`;
        const preCtx = this.makeContext(
          node,
          fires,
          preKey,
          pending,
          dryRun,
          secrets,
          workflow,
          log,
          requestShutdown,
          "",
          this.stateStore,
        );

        const resolvedEither = yield* resolveValueForNode(node.with_, triggers, preCtx, node).pipe(
          Effect.mapError((error: TemplateError) => new DriverError({ reason: error.reason })),
          Effect.either,
        );

        if (Either.isLeft(resolvedEither)) {
          const fail = Event.nodeFailed(node.id, resolvedEither.left.reason);
          this.observer.onNodeEnd(node, [fail], new Error(resolvedEither.left.reason), span);
          return [fail];
        }

        const resolvedWith = resolvedEither.right as Readonly<Record<string, unknown>>;

        if (!node.driver) {
          this.observer.onNodeEnd(node, [], null, span);
          return [];
        }
        const driver = yield* this.registry.resolve(node.driver);
        const inputSchema = node.input ?? driver.input ?? null;
        if (inputSchema !== null) {
          const inputErrors = validateObject(inputSchema, resolvedWith);
          if (inputErrors.length > 0) {
            const invalid = Event.eventInvalid("node.input", node.id, inputErrors, resolvedWith);
            if (workflow.strict) {
              const fail = Event.nodeFailed(
                node.id,
                `input validation failed: ${inputErrors.join("; ")}`,
              );
              this.observer.onNodeEnd(node, [invalid, fail], null, span);
              return [invalid, fail];
            }
            this.observer.onNodeEnd(node, [invalid], null, span);
            return [invalid];
          }
        }

        let stateKey = "";
        let nodeState: StateStore = this.stateStore;
        const stateKeyTemplate = node.state["key"];
        if (typeof stateKeyTemplate === "string" && stateKeyTemplate.length > 0) {
          stateKey = yield* resolveTemplate(stateKeyTemplate, triggers, preCtx, node).pipe(
            Effect.mapError((error: TemplateError) => new DriverError({ reason: error.reason })),
          );
          const record = this.stateStore.get(stateKey);
          const initial: Record<string, unknown> =
            record !== null && typeof record === "object" && !Array.isArray(record)
              ? { ...(record as Record<string, unknown>) }
              : {};
          nodeState = new ScopedStateStore(initial);
          const skipIf = node.state["skip_if"];
          if (typeof skipIf === "string" && skipIf.length > 0) {
            const skipCtx = this.makeContext(
              node,
              fires,
              `${node.id}:skip`,
              pending,
              dryRun,
              secrets,
              workflow,
              log,
              requestShutdown,
              "",
              nodeState,
            );
            const rendered = yield* resolveTemplate(skipIf, triggers, skipCtx, node).pipe(
              Effect.mapError((error: TemplateError) => new DriverError({ reason: error.reason })),
            );
            if (["true", "yes", "1", "ok"].includes(rendered.trim().toLowerCase())) {
              const skipped = new Event(
                "node.skipped",
                { node: node.id, state_key: stateKey },
                node.id,
              );
              this.observer.onNodeEnd(node, [skipped], null, span);
              return [skipped];
            }
          }
        }

        const capChecker = new CapabilityChecker(workflow.capabilities, workflow.strict);
        const effective = capChecker.merge(node.capabilities);
        const check = new CapabilityChecker(effective, workflow.strict);
        const violations = check.check(node.driver, resolvedWith);

        if (violations.length > 0) {
          const events: Array<Event> = [Event.capabilityViolation(node.id, violations)];
          if (workflow.strict) {
            events.push(
              Event.nodeFailed(node.id, `capability violations: ${violations.join("; ")}`),
            );
          }
          this.observer.onNodeEnd(node, events, null, span);
          return events;
        }

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
            span.getTraceparent(),
            nodeState,
          );

        const driverEvents = node.fanIn
          ? ([] as ReadonlyArray<Event>)
          : yield* this.runWithRetry(node, triggers, makeCtx, dryRun, driver, workflow);

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
            const scheduled = Effect.sleep(Duration.seconds(interval)).pipe(
              Effect.andThen(
                Effect.sync(() => {
                  pending.current.push(new Event(tick, { poll: true }, node.id));
                }),
              ),
            );
            yield* Effect.fork(scheduled);
          }
        }

        const all = this.buildNodeEvents(node, triggers, driverEvents);

        if (workflow.strict) {
          const allowedEmits = new Set(node.emits);
          for (const event of all) {
            if (
              event.type === "node.failed" ||
              event.type === "circuit.open" ||
              event.type === "node.skipped"
            )
              continue;
            if (!allowedEmits.has(event.type)) {
              const fail = Event.nodeFailed(
                node.id,
                `emitted '${event.type}' not declared in 'emits'`,
              );
              this.observer.onNodeEnd(node, [fail], null, span);
              return [fail];
            }
          }
        }

        if (stateKey.length > 0 && !driverEvents.some((event) => event.type === "node.failed")) {
          this.stateStore.set(stateKey, nodeState.toRecord());
        }

        this.observer.onNodeEnd(node, all, null, span);
        return all;
      });

      const reserved = yield* this.resourceGate.acquire(this.resourceWeight(node));
      const result = yield* run.pipe(
        Effect.ensuring(Effect.sync(() => this.resourceGate.release(reserved))),
      );
      return result;
    });
  }

  private buildNodeEvents(
    node: Node,
    triggers: ReadonlyArray<Event>,
    driverEvents: ReadonlyArray<Event>,
  ): ReadonlyArray<Event> {
    const out: Array<Event> = [];
    if (node.fanIn) {
      const fanInEmit = node.fanIn?.["emit"];
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
      const interval =
        typeof node.with_.interval === "number"
          ? node.with_.interval
          : typeof node.poll?.interval === "number"
            ? node.poll.interval
            : 0;
      if (interval <= 0 && !out.some((event) => event.type === tick)) {
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
    driver: Driver,
    workflow: Workflow,
  ) {
    return Effect.gen(this, function* (this: RunnerLive, _resume: Adapter) {
      let now = yield* Effect.clockWith((clock) => clock.currentTimeMillis);
      if (this.isCircuitOpen(node, now)) {
        return [Event.nodeFailed(node.id, "circuit open")];
      }

      const retryCfg = node.retry;
      const maxAttempts = Math.max(1, Number(retryCfg["max_attempts"] ?? 1));
      let lastError: DriverError | null = null;

      for (let attempt = 1; attempt <= maxAttempts; attempt++) {
        const idempotencyKey = `${node.id}:${++this.idempotencyCounter}`;
        const ctx = makeCtx(idempotencyKey);
        const result = yield* this.runDriver(node, driver, triggers, ctx, dryRun).pipe(
          Effect.either,
        );
        if (Either.isRight(result)) {
          this.recordSuccess(node);
          this.retryCounts.delete(node.id);
          return this.validateOutput(node, driver, result.right, workflow);
        }
        lastError = result.left;

        now = yield* Effect.clockWith((clock) => clock.currentTimeMillis);
        if (this.recordFailure(node, now)) {
          const onTripRaw = node.circuitBreaker?.["on_trip"];
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
          const baseDelay = Number(retryCfg["delay"] ?? 1);
          const backoffRaw = retryCfg["backoff"];
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

      const fail = Event.nodeFailed(node.id, lastError?.reason ?? "unknown error");
      yield* this.dlq.enqueue(fail, "retry exhausted");
      return [fail];
    });
  }

  private runDriver(
    node: Node,
    driver: Driver,
    triggers: ReadonlyArray<Event>,
    ctx: ExecutionContext,
    dryRun: boolean,
  ) {
    return Effect.gen(this, function* (this: RunnerLive, _resume: Adapter) {
      if (dryRun) {
        return [new Event(node.emits[0] ?? `${node.id}.done`, { dry_run: true }, node.id)];
      }
      return yield* driver.invoke(node, triggers, ctx);
    });
  }

  private validateOutput(
    node: Node,
    driver: Driver,
    events: ReadonlyArray<Event>,
    workflow: Workflow,
  ): ReadonlyArray<Event> {
    const schemas = resolveOutputSchemas(driver.output ?? [], node.output);
    if (schemas.length === 0) return events;
    const out: Array<Event> = [];
    let hasInvalid = false;
    for (const event of events) {
      const schema = matchingOutputSchema(schemas, event.type);
      if (schema === null) {
        out.push(event);
        continue;
      }
      const errors = validateObject(schema, event.payload);
      if (errors.length === 0) {
        out.push(event);
        continue;
      }
      hasInvalid = true;
      out.push(Event.eventInvalid(event.type, node.id, errors, event.payload));
    }
    if (hasInvalid && workflow.strict) {
      out.push(Event.nodeFailed(node.id, "driver output validation failed"));
    }
    return out;
  }

  private isCircuitOpen(node: Node, now: number): boolean {
    const cb = this.circuitStates.get(node.id);
    if (!cb || !cb.open) return false;
    const resetAfter = Number(node.circuitBreaker?.["reset_after"] ?? 60) * 1000;
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
    const threshold = Number(node.circuitBreaker?.["threshold"] ?? 5);
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
    traceparent: string,
    state: StateStore,
  ): ExecutionContext {
    const missingSecrets: Array<string> = [];
    const secretProxy = new Proxy(secrets, {
      get: (target, prop) => {
        if (typeof prop === "symbol" || prop === "then") return undefined;
        const key = prop as string;
        if (key in target) return (target as Record<string, string>)[key];
        if (workflow.strict) missingSecrets.push(key);
        return "";
      },
    });
    const capabilities = { ...workflow.capabilities, ...node.capabilities };
    return {
      config: workflow.config,
      secrets: secretProxy as Readonly<Record<string, string>>,
      log,
      emit: (event: Event) => {
        pending.current.push(event);
      },
      inject: (nodes: ReadonlyArray<Record<string, unknown>>) => {
        this.bus.inject(nodes);
      },
      dryRun,
      idempotencyKey,
      traceparent,
      state,
      fires,
      requestShutdown,
      getSecret: (key: string, default_ = ""): string => {
        if (key in secrets) return secrets[key];
        if (workflow.strict && (default_ === undefined || default_ === "")) {
          missingSecrets.push(key);
        }
        return default_;
      },
      resources: node.resources,
      capabilities,
      workflowCapabilities: workflow.capabilities,
      strict: workflow.strict,
      missingSecrets,
      pythonImports: workflow.pythonImports,
    };
  }
}

export const RunnerLiveLayer = Layer.effect(
  Runner,
  Effect.gen(function* () {
    const bus = yield* EventBus;
    const registry = yield* DriverRegistry;
    const observer = yield* Observer;
    const stateStore = yield* StateStore;
    return new RunnerLive(
      bus,
      registry,
      observer,
      new ResourceGate(),
      noOpDeadLetterQueue,
      stateStore,
    );
  }),
);
