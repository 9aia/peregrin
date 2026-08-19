import crypto from "node:crypto";

export type TriggerMode = "any" | "all";

export class Event {
  constructor(
    readonly type: string,
    readonly payload: Readonly<Record<string, unknown>> = {},
    readonly source: string = "",
    readonly id: string = crypto.randomUUID().slice(0, 12),
    readonly ts: number = Date.now(),
  ) {}

  static start(): Event {
    return new Event("start", {}, "__runner__");
  }

  static done(): Event {
    return new Event("workflow.done", {}, "__runner__");
  }

  static cancel(reason = ""): Event {
    return new Event("workflow.cancel", { reason }, "__runner__");
  }

  static nodeFailed(nodeId: string, error: string): Event {
    return new Event("node.failed", { error, node: nodeId }, nodeId);
  }

  static nodeExhausted(nodeId: string, maxFires: number | "unlimited"): Event {
    return new Event("node.exhausted", { node: nodeId, maxFires }, nodeId);
  }

  static workflowFailed(nodeId: string, error: string): Event {
    return new Event("workflow.failed", { node: nodeId, error }, "__runner__");
  }

  static eventInvalid(
    eventType: string,
    source: string,
    errors: ReadonlyArray<string>,
    payload: Readonly<Record<string, unknown>>,
  ): Event {
    return new Event("event.invalid", { eventType, errors, payload }, source);
  }

  static capabilityViolation(nodeId: string, violations: ReadonlyArray<string>): Event {
    return new Event("capability.violation", { node: nodeId, violations }, nodeId);
  }
}

export class EventSchema {
  constructor(
    readonly type: string,
    readonly fields: Readonly<Record<string, string>> = {},
    readonly schema: Readonly<Record<string, unknown>> = {},
  ) {}

  compiledSchema(): Readonly<Record<string, unknown>> | null {
    if (Object.keys(this.schema).length > 0) return this.schema;
    if (Object.keys(this.fields).length === 0) return null;
    const properties: Record<string, unknown> = {};
    const required: string[] = [];
    for (const [name, typeStr] of Object.entries(this.fields)) {
      const optional = typeStr.endsWith("?");
      const jsonType = typeStr.replace(/\?$/, "");
      if (
        ["string", "integer", "number", "boolean", "array", "object", "null"].includes(jsonType)
      ) {
        properties[name] = { type: jsonType };
      } else {
        properties[name] = { type: "string" };
      }
      if (!optional) required.push(name);
    }
    const compiled: Record<string, unknown> = { type: "object", properties };
    if (required.length > 0) compiled.required = required;
    return compiled;
  }
}

function parseMaxFires(value: unknown, defaultValue: number | "unlimited"): number | "unlimited" {
  if (value === "unlimited") return "unlimited";
  if (typeof value === "number" && Number.isInteger(value) && value > 0) return value;
  if (value === 0) return "unlimited";
  return defaultValue;
}

export class Node {
  constructor(
    readonly id: string,
    readonly driver: string = "",
    readonly handler: string = "",
    readonly on: ReadonlyArray<string> = [],
    readonly emits: ReadonlyArray<string> = [],
    readonly mode: TriggerMode = "any",
    readonly with_: Readonly<Record<string, unknown>> = {},
    readonly maxFires: number | "unlimited" = 1,
    readonly until: ReadonlyArray<string> = [],
    readonly retireOn: ReadonlyArray<string> = [],
    readonly retry: Readonly<Record<string, unknown>> = {},
    readonly circuitBreaker: Readonly<Record<string, unknown>> = {},
    readonly poll: Readonly<Record<string, unknown>> | null = null,
    readonly fanIn: Readonly<Record<string, unknown>> | null = null,
    readonly forward: ReadonlyArray<string> = [],
    readonly resources: Readonly<Record<string, unknown>> = {},
    readonly capabilities: Readonly<Record<string, unknown>> = {},
  ) {}

  static fromDict(raw: Readonly<Record<string, unknown>>): Node {
    const poll = raw.poll as Readonly<Record<string, unknown>> | undefined;
    if (poll) {
      const nodeId = String(raw.id);
      const tick = `${nodeId}.tick`;
      const on = Array.from<string>((raw.on as Array<string>) ?? ["start"]);
      const emits = Array.from<string>((raw.emits as Array<string>) ?? []);
      if (!on.includes(tick)) on.push(tick);
      if (!emits.includes(tick)) emits.push(tick);
      const maxFires = parseMaxFires(raw.max_fires, "unlimited");
      const until = Array.from<string>((raw.until as Array<string>) ?? []);
      const pollUntil = Array.from<string>((poll.until as Array<string>) ?? []);
      for (const e of pollUntil) if (!until.includes(e)) until.push(e);
      const with_: Record<string, unknown> = {
        ...(raw.with as Record<string, unknown>),
      };
      if (typeof poll.interval === "number" && !("interval" in with_)) {
        with_.interval = poll.interval;
      }
      return new Node(
        nodeId,
        String(raw.driver ?? ""),
        String(raw.handler ?? ""),
        on,
        emits,
        (raw.mode as TriggerMode) ?? "any",
        with_,
        maxFires,
        until,
        Array.from<string>((raw.retire_on as Array<string>) ?? []),
        (raw.retry as Record<string, unknown>) ?? {},
        (raw.circuit_breaker as Record<string, unknown>) ?? {},
        poll,
        (raw.fan_in as Record<string, unknown>) ?? null,
        Array.from<string>((raw.forward as Array<string>) ?? []),
        (raw.resources as Record<string, unknown>) ?? {},
        (raw.capabilities as Record<string, unknown>) ?? {},
      );
    }
    return new Node(
      String(raw.id),
      String(raw.driver ?? ""),
      String(raw.handler ?? ""),
      Array.from<string>((raw.on as Array<string>) ?? []),
      Array.from<string>((raw.emits as Array<string>) ?? []),
      (raw.mode as TriggerMode) ?? "any",
      (raw.with as Record<string, unknown>) ?? {},
      parseMaxFires(raw.max_fires, 1),
      Array.from<string>((raw.until as Array<string>) ?? []),
      Array.from<string>((raw.retire_on as Array<string>) ?? []),
      (raw.retry as Record<string, unknown>) ?? {},
      (raw.circuit_breaker as Record<string, unknown>) ?? {},
      (raw.poll as Record<string, unknown>) ?? null,
      (raw.fan_in as Record<string, unknown>) ?? null,
      Array.from<string>((raw.forward as Array<string>) ?? []),
      (raw.resources as Record<string, unknown>) ?? {},
      (raw.capabilities as Record<string, unknown>) ?? {},
    );
  }

  toDict(): Record<string, unknown> {
    const d: Record<string, unknown> = {
      id: this.id,
      driver: this.driver,
      handler: this.handler,
      on: [...this.on],
      emits: [...this.emits],
      mode: this.mode,
      with: this.with_,
    };
    const defaultFires = this.poll ? "unlimited" : 1;
    if (this.maxFires !== defaultFires) d.max_fires = this.maxFires;
    if (this.until.length > 0) d.until = [...this.until];
    if (this.retireOn.length > 0) d.retire_on = [...this.retireOn];
    if (Object.keys(this.retry).length > 0) d.retry = this.retry;
    if (Object.keys(this.circuitBreaker).length > 0) d.circuit_breaker = this.circuitBreaker;
    if (this.poll) d.poll = this.poll;
    if (this.fanIn) d.fan_in = this.fanIn;
    if (this.forward.length > 0) d.forward = [...this.forward];
    if (Object.keys(this.resources).length > 0) d.resources = this.resources;
    if (Object.keys(this.capabilities).length > 0) d.capabilities = this.capabilities;
    return d;
  }
}

export class Workflow {
  constructor(
    readonly name: string = "",
    readonly events: ReadonlyArray<EventSchema> = [],
    readonly nodes: ReadonlyArray<Node> = [],
    readonly config: Readonly<Record<string, unknown>> = {},
    readonly imports: ReadonlyArray<string> = [],
    readonly stopOn: ReadonlyArray<string> = [],
    readonly payloadSchemas: Readonly<Record<string, Readonly<Record<string, unknown>>>> = {},
    readonly capabilities: Readonly<Record<string, unknown>> = {},
    readonly strict: boolean = false,
  ) {}

  static fromDict(raw: Readonly<Record<string, unknown>>): Workflow {
    return new Workflow(
      String(raw.name ?? ""),
      ((raw.events as Array<Record<string, unknown>>) ?? []).map(
        (e) =>
          new EventSchema(
            String(e.type),
            (e.fields as Record<string, string>) ?? {},
            (e.schema as Record<string, unknown>) ?? {},
          ),
      ),
      ((raw.nodes as Array<Record<string, unknown>>) ?? []).map(Node.fromDict),
      (raw.config as Record<string, unknown>) ?? {},
      Array.from<string>((raw.include as Array<string>) ?? (raw.includes as Array<string>) ?? []),
      Array.from<string>((raw.stop_on as Array<string>) ?? []),
      (raw.payload_schemas as Record<string, Record<string, unknown>>) ?? {},
      (raw.capabilities as Record<string, unknown>) ?? {},
      Boolean(raw.strict),
    );
  }

  toDict(): Record<string, unknown> {
    const d: Record<string, unknown> = {
      name: this.name,
      events: this.events.map((e) => ({
        type: e.type,
        fields: e.fields,
        schema: e.schema,
      })),
      nodes: this.nodes.map((n) => n.toDict()),
      config: this.config,
    };
    if (this.imports.length > 0) d.include = [...this.imports];
    if (this.stopOn.length > 0) d.stop_on = [...this.stopOn];
    if (Object.keys(this.capabilities).length > 0) d.capabilities = this.capabilities;
    if (this.strict) d.strict = true;
    return d;
  }

  merge(other: Workflow): void {
    const existingTypes = new Set(this.events.map((e) => e.type));
    for (const event of other.events) {
      if (!existingTypes.has(event.type)) {
        (this.events as EventSchema[]).push(event);
      }
    }
    for (const [key, val] of Object.entries(other.config)) {
      if (!(key in this.config)) {
        (this.config as Record<string, unknown>)[key] = val;
      }
    }
  }
}

export interface StateStore {
  get(key: string): unknown;
  set(key: string, value: unknown): void;
  toRecord(): Readonly<Record<string, unknown>>;
}

export interface Runner {
  requestShutdown(): void;
}

export interface ExecutionContext {
  readonly config: Readonly<Record<string, unknown>>;
  readonly secrets: Readonly<Record<string, string>>;
  readonly log: ReadonlyArray<Event>;
  readonly emit: (event: Event) => void;
  readonly inject: (nodes: ReadonlyArray<Record<string, unknown>>) => void;
  readonly dryRun: boolean;
  readonly idempotencyKey: string;
  readonly traceparent: string;
  readonly state: StateStore;
  readonly fires: number;
  readonly requestShutdown: () => void;
  readonly getSecret: (key: string, default_?: string) => string;
  readonly strict: boolean;
}
