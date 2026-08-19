# Write a custom store

Implement the `EventStore` interface from `ac-core` and provide it as an Effect `Layer`.

## The `EventStore` interface

```ts
import { Event, EventStore } from "ac-core";

interface EventStore {
  append(event: Event): void;
  all(): ReadonlyArray<Event>;
  byType(eventType: string): ReadonlyArray<Event>;
  seenTypes(): ReadonlySet<string>;
  latest(): Event | null;
  clear(): void;
  close(): void;
}
```

## SQLite store

```ts
import sqlite3 from "sqlite3";
import { Event, EventStore } from "ac-core";
import { Layer } from "effect";

class SQLiteStore implements EventStore {
  private db: sqlite3.Database;

  constructor(path: string) {
    this.db = new sqlite3.Database(path);
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS events (
        type TEXT, payload TEXT, source TEXT, id TEXT, ts REAL
      )
    `);
  }

  append(event: Event): void {
    const json = JSON.stringify(event.payload);
    this.db.run("INSERT INTO events VALUES (?, ?, ?, ?, ?)", [
      event.type,
      json,
      event.source,
      event.id,
      event.ts,
    ]);
  }

  all(): ReadonlyArray<Event> {
    return this.query("SELECT * FROM events ORDER BY ts");
  }

  byType(eventType: string): ReadonlyArray<Event> {
    return this.query("SELECT * FROM events WHERE type = ? ORDER BY ts", [eventType]);
  }

  private query(sql: string, params: unknown[] = []): Array<Event> {
    const rows: Array<{
      type: string;
      payload: string;
      source: string;
      id: string;
      ts: number;
    }> = [];
    this.db.each(sql, params, (err, row) => {
      if (!err) rows.push(row as any);
    });
    return rows.map((r) => new Event(r.type, JSON.parse(r.payload), r.source, r.id, r.ts));
  }

  seenTypes(): ReadonlySet<string> {
    const types = new Set<string>();
    this.db.each("SELECT DISTINCT type FROM events", (err, row) => {
      if (!err) types.add((row as any).type);
    });
    return types;
  }

  latest(): Event | null {
    const events = this.all();
    return events.at(-1) ?? null;
  }

  clear(): void {
    this.db.run("DELETE FROM events");
  }

  close(): void {
    this.db.close();
  }
}
```

Use it:

```ts
import { Effect } from "effect";
import { Runner } from "ac-engine";
import { DefaultsLive } from "ac-cli-defaults";

const SQLiteLive = Layer.succeed(EventStore, new SQLiteStore("/data/workflow.db"));

const program = Effect.gen(function* () {
  const runner = yield* Runner;
  const log = yield* runner.run(workflow);
  return log;
}).pipe(Effect.provide(DefaultsLive), Effect.provide(SQLiteLive));

Effect.runPromise(program);
```

## See also

- [Stores](../explanation/stores.md) — the adapter concept
- [JSON store](../reference/json-store.md)
- [Memory store](../reference/memory-store.md)
- [Use the programmatic API](programmatic-api.md)
