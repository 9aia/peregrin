import { Event, EventStore } from "ac-core";
import { Data, Effect, Layer } from "effect";
import fs from "node:fs";
import path from "node:path";

export class JsonStoreError extends Data.TaggedError("JsonStoreError")<{
  readonly reason: string;
}> {}

export class JsonStore implements EventStore {
  private events: Array<Event> = [];
  private eventsByType: Record<string, Array<Event>> = {};
  private seen: Set<string> = new Set();

  private constructor(readonly filePath: string) {
    if (fs.existsSync(this.filePath)) {
      const content = fs.readFileSync(this.filePath, "utf8");
      for (const line of content.split("\n")) {
        const trimmed = line.trim();
        if (trimmed === "") continue;
        const raw = JSON.parse(trimmed) as Record<string, unknown>;
        this.index(this.toEvent(raw));
      }
      fs.chmodSync(this.filePath, 0o600);
    }
  }

  private toEvent(raw: Record<string, unknown>): Event {
    return new Event(
      String(raw.type ?? ""),
      (raw.payload ?? {}) as Record<string, unknown>,
      String(raw.source ?? ""),
      String(raw.id ?? ""),
      typeof raw.ts === "number" ? raw.ts : 0,
    );
  }

  private index(event: Event): void {
    this.events.push(event);
    this.seen.add(event.type);
    (this.eventsByType[event.type] ??= []).push(event);
  }

  private serialize(event: Event): string {
    return JSON.stringify({
      type: event.type,
      payload: event.payload,
      source: event.source,
      id: event.id,
      ts: event.ts,
    });
  }

  append(event: Event): void {
    this.index(event);
    const dir = path.dirname(this.filePath);
    if (!fs.existsSync(dir)) {
      fs.mkdirSync(dir, { recursive: true });
    }
    if (!fs.existsSync(this.filePath)) {
      fs.writeFileSync(this.filePath, "", { mode: 0o600 });
    }
    fs.appendFileSync(this.filePath, this.serialize(event) + "\n");
  }

  all(): ReadonlyArray<Event> {
    return this.events;
  }

  byType(eventType: string): ReadonlyArray<Event> {
    return this.eventsByType[eventType] ?? [];
  }

  seenTypes(): ReadonlySet<string> {
    return new Set(this.seen);
  }

  latest(): Event | null {
    return this.events.at(-1) ?? null;
  }

  clear(): void {
    this.events = [];
    this.eventsByType = {};
    this.seen = new Set();
    if (fs.existsSync(this.filePath)) {
      fs.rmSync(this.filePath, { force: true });
    }
  }

  close(): void {}

  static make(filePath: string): Effect.Effect<EventStore, JsonStoreError, never> {
    return Effect.try(() => new JsonStore(filePath) as EventStore).pipe(
      Effect.mapError((error) => new JsonStoreError({ reason: String(error) })),
    );
  }
}

export const Live = (filePath: string): Layer.Layer<EventStore, JsonStoreError, never> =>
  Layer.effect(EventStore, JsonStore.make(filePath));
