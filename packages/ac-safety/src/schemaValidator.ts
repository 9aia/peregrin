import type { Event, EventSchema } from "handl-core";

const SYNTHETIC_TYPES = new Set([
  "start",
  "workflow.done",
  "workflow.cancel",
  "workflow.cancelled",
  "workflow.failed",
  "workflow.cycle_detected",
]);

type JsonType = "string" | "integer" | "number" | "boolean" | "array" | "object" | "null";

function asStringArray(val: unknown): ReadonlyArray<string> {
  if (!Array.isArray(val)) return [];
  return val.map(String);
}

function typeMatches(value: unknown, jsonType: JsonType): boolean {
  switch (jsonType) {
    case "string":
      return typeof value === "string";
    case "integer":
      return Number.isInteger(value);
    case "number":
      return typeof value === "number";
    case "boolean":
      return typeof value === "boolean";
    case "array":
      return Array.isArray(value);
    case "object":
      return value !== null && typeof value === "object" && !Array.isArray(value);
    case "null":
      return value === null;
    default:
      return true;
  }
}

function formatError(path: string, message: string): string {
  return `${path}: ${message}`;
}

export function validateRecord(
  schema: Readonly<Record<string, unknown>>,
  payload: Readonly<Record<string, unknown>>,
): ReadonlyArray<string> {
  const errors: string[] = [];
  const required = asStringArray(schema["required"]);
  for (const name of required) {
    if (!(name in payload)) {
      errors.push(formatError(name, "missing required property"));
    }
  }

  const properties = (schema["properties"] ?? {}) as Record<string, { type?: string }>;
  for (const [name, spec] of Object.entries(properties)) {
    if (!(name in payload)) continue;
    const expected = spec.type as JsonType | undefined;
    if (expected !== undefined && !typeMatches(payload[name], expected)) {
      errors.push(formatError(name, `expected type ${expected}`));
    }
  }

  return errors;
}

export function validateObject(
  schema: EventSchema,
  value: Readonly<Record<string, unknown>>,
): ReadonlyArray<string> {
  const compiled = schema.compiledSchema();
  if (compiled === null) return [];
  return validateRecord(compiled as Record<string, unknown>, value);
}

export class SchemaValidator {
  private readonly _validators: Record<string, Readonly<Record<string, unknown>>> = {};

  constructor(schemas: ReadonlyArray<EventSchema>) {
    for (const schema of schemas) {
      const compiled = schema.compiledSchema();
      if (compiled === null) continue;
      this._validators[schema.type] = compiled;
    }
  }

  hasSchema(eventType: string): boolean {
    return eventType in this._validators;
  }

  validate(event: Event): ReadonlyArray<string> {
    if (SYNTHETIC_TYPES.has(event.type)) return [];
    const schema = this._validators[event.type];
    if (schema === undefined) return [];

    const payload = event.payload;

    if (typeof payload !== "object" || payload === null) {
      return [formatError("(root)", "payload must be an object")];
    }

    return validateRecord(schema, payload as Record<string, unknown>);
  }
}

export function buildValidator(schemas: ReadonlyArray<EventSchema>): SchemaValidator | null {
  let hasAny = false;
  for (const schema of schemas) {
    if (schema.compiledSchema() !== null) {
      hasAny = true;
      break;
    }
  }
  if (!hasAny) return null;
  return new SchemaValidator(schemas);
}
