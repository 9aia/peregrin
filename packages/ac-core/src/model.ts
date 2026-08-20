import crypto from "node:crypto";
import type { StateStore } from "./ports.js";

function coerceToString(value: unknown, fallback = ""): string {
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return fallback;
}

export interface PythonImport {
  readonly file: string;
  readonly as: string;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function parsePythonImports(value: unknown): ReadonlyArray<PythonImport> {
  if (value === undefined || value === null) return [];
  const items = Array.isArray(value) ? value : [value];
  const out: Array<PythonImport> = [];
  for (const item of items) {
    if (typeof item === "string") {
      out.push({ file: item, as: "" });
    } else if (isRecord(item) && typeof item.file === "string") {
      out.push({ file: item.file, as: typeof item.as === "string" ? item.as : "" });
    }
  }
  return out;
}

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
    readonly annotations: Readonly<Record<string, unknown>> = {},
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

  static fromDict(raw: Readonly<Record<string, unknown>>): EventSchema {
    return new EventSchema(
      coerceToString(raw.type),
      (raw.fields as Record<string, string>) ?? {},
      (raw.schema as Record<string, unknown>) ?? {},
      (raw.annotations as Record<string, unknown>) ?? {},
    );
  }

  toDict(): Record<string, unknown> {
    const d: Record<string, unknown> = {
      type: this.type,
      fields: this.fields,
      schema: this.schema,
    };
    if (Object.keys(this.annotations).length > 0) d.annotations = this.annotations;
    return d;
  }
}

function parseMaxFires(value: unknown, defaultValue: number | "unlimited"): number | "unlimited" {
  if (value === "unlimited") return "unlimited";
  if (typeof value === "number" && Number.isInteger(value) && value > 0) return value;
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
    readonly state: Readonly<Record<string, unknown>> = {},
    readonly annotations: Readonly<Record<string, unknown>> = {},
  ) {}

  static fromDict(raw: Readonly<Record<string, unknown>>): Node {
    const poll = raw.poll as Readonly<Record<string, unknown>> | undefined;
    const state = (raw.state as Record<string, unknown>) ?? {};
    const annotations = (raw.annotations as Record<string, unknown>) ?? {};
    if (poll) {
      const nodeId = coerceToString(raw.id);
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
        coerceToString(raw.driver),
        coerceToString(raw.handler),
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
        state,
        annotations,
      );
    }
    return new Node(
      coerceToString(raw.id),
      coerceToString(raw.driver),
      coerceToString(raw.handler),
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
      state,
      annotations,
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
    const defaultFires = 1;
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
    if (Object.keys(this.state).length > 0) d.state = this.state;
    if (Object.keys(this.annotations).length > 0) d.annotations = this.annotations;
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
    readonly pythonImports: ReadonlyArray<PythonImport> = [],
    readonly stopOn: ReadonlyArray<string> = [],
    readonly payloadSchemas: Readonly<Record<string, Readonly<Record<string, unknown>>>> = {},
    readonly capabilities: Readonly<Record<string, unknown>> = {},
    readonly strict: boolean = false,
    readonly state: Readonly<Record<string, unknown>> = {},
    readonly annotations: Readonly<Record<string, unknown>> = {},
  ) {}

  static fromDict(raw: Readonly<Record<string, unknown>>): Workflow {
    return new Workflow(
      coerceToString(raw.name),
      ((raw.events as Array<Record<string, unknown>>) ?? []).map((e) => EventSchema.fromDict(e)),
      ((raw.nodes as Array<Record<string, unknown>>) ?? []).map((node) => Node.fromDict(node)),
      (raw.config as Record<string, unknown>) ?? {},
      Array.from<string>((raw.include as Array<string>) ?? (raw.includes as Array<string>) ?? []),
      parsePythonImports(raw.import),
      Array.from<string>((raw.stop_on as Array<string>) ?? []),
      (raw.payload_schemas as Record<string, Record<string, unknown>>) ?? {},
      (raw.capabilities as Record<string, unknown>) ?? {},
      Boolean(raw.strict),
      (raw.state as Record<string, unknown>) ?? {},
      (raw.annotations as Record<string, unknown>) ?? {},
    );
  }

  toDict(): Record<string, unknown> {
    const d: Record<string, unknown> = {
      name: this.name,
      events: this.events.map((e) => e.toDict()),
      nodes: this.nodes.map((n) => n.toDict()),
      config: this.config,
    };
    if (this.imports.length > 0) d.include = [...this.imports];
    if (this.pythonImports.length > 0) {
      d.import = this.pythonImports.map((i) =>
        i.as.length > 0 ? { file: i.file, as: i.as } : i.file,
      );
    }
    if (this.stopOn.length > 0) d.stop_on = [...this.stopOn];
    if (Object.keys(this.capabilities).length > 0) d.capabilities = this.capabilities;
    if (this.strict) d.strict = true;
    if (Object.keys(this.state).length > 0) d.state = this.state;
    if (Object.keys(this.annotations).length > 0) d.annotations = this.annotations;
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
  readonly missingSecrets: Array<string>;
  readonly capabilities: Readonly<Record<string, unknown>>;
  readonly resources: Readonly<Record<string, unknown>>;
  readonly workflowCapabilities: Readonly<Record<string, unknown>>;
  readonly pythonImports?: ReadonlyArray<PythonImport>;
}
