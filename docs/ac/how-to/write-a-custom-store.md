# Write a custom store

Implement the `EventStore` protocol and pass it to the runner.

## SQLite store

```python
import json
import sqlite3
from agent_campaign.model import Event

class SQLiteStore:
    def __init__(self, path: str = ":memory:"):
        self._conn = sqlite3.connect(path)
        self._conn.execute("""
            CREATE TABLE IF NOT EXISTS events (
                type TEXT, payload TEXT, source TEXT, id TEXT, ts REAL
            )
        """)
        self._conn.commit()

    def append(self, event: Event) -> None:
        self._conn.execute(
            "INSERT INTO events VALUES (?,?,?,?,?)",
            (event.type, json.dumps(event.payload), event.source, event.id, event.ts),
        )
        self._conn.commit()

    def all(self) -> list[Event]:
        rows = self._conn.execute("SELECT * FROM events ORDER BY ts").fetchall()
        return [Event(type=r[0], payload=json.loads(r[1]), source=r[2],
                      id=r[3], ts=r[4]) for r in rows]

    def by_type(self, event_type: str) -> list[Event]:
        rows = self._conn.execute(
            "SELECT * FROM events WHERE type=? ORDER BY ts", (event_type,)
        ).fetchall()
        return [Event(type=r[0], payload=json.loads(r[1]), source=r[2],
                      id=r[3], ts=r[4]) for r in rows]

    def seen_types(self) -> set[str]:
        return {r[0] for r in self._conn.execute("SELECT DISTINCT type FROM events")}

    def latest(self) -> Event | None:
        events = self.all()
        return events[-1] if events else None

    def clear(self) -> None:
        self._conn.execute("DELETE FROM events")
        self._conn.commit()

    def close(self) -> None:
        self._conn.close()
```

Use it:

```python
runner = Runner(wf, store=SQLiteStore("/data/workflow.db"))
```

## Postgres store (sketch)

```python
import psycopg2

class PostgresStore:
    def __init__(self, dsn: str):
        self._conn = psycopg2.connect(dsn)
        self._conn.execute("""
            CREATE TABLE IF NOT EXISTS events (
                type TEXT, payload JSONB, source TEXT, id TEXT, ts REAL
            )
        """)

    def append(self, event: Event) -> None:
        self._conn.execute(
            "INSERT INTO events VALUES (%s, %s, %s, %s, %s)",
            (event.type, json.dumps(event.payload), event.source, event.id, event.ts),
        )
        self._conn.commit()

    # ... implement all(), by_type(), seen_types(), latest(), clear(), close()
```

## See also

- [Stores](../explanation/stores.md) — the adapter concept
- [JSON store](../reference/json-store.md), [Memory store](../reference/memory-store.md)
- [Use the programmatic API](programmatic-api.md)
