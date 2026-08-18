export const REDACTED = "***REDACTED***";

export function redactValue(value: unknown, secretValues: ReadonlyArray<string>): unknown {
  if (typeof value === "string") {
    for (const secret of secretValues) {
      if (secret.length > 0 && value.includes(secret)) return REDACTED;
    }
    return value;
  }
  if (Array.isArray(value)) {
    return value.map((v) => redactValue(v, secretValues));
  }
  if (value !== null && typeof value === "object") {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(value)) out[k] = redactValue(v, secretValues);
    return out;
  }
  return value;
}

export function redactPayload(
  payload: Readonly<Record<string, unknown>>,
  secrets: Readonly<Record<string, string>>,
): Record<string, unknown> {
  const secretValues = Object.values(secrets).filter((v) => v.length > 0);
  return redactValue(payload, secretValues) as Record<string, unknown>;
}
