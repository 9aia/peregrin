# Write a custom observer

Implement the `Observer` protocol and pass it to the runner.

## Datadog observer

```python
import time
from agent_campaign.observers.base import NoopSpan

class DatadogObserver:
    def __init__(self):
        from datadog import statsd
        self._statsd = statsd

    def on_workflow_start(self, workflow):
        self._start = time.time()
        self._name = workflow.name

    def on_workflow_end(self, event_count, cancelled, cycle, failed=False):
        duration = time.time() - self._start
        tags = [f"workflow:{self._name}"]
        self._statsd.histogram("workflow.duration", duration, tags=tags)
        self._statsd.increment("workflow.events", event_count, tags=tags)
        if failed:
            self._statsd.increment("workflow.failures", tags=tags)
        if cancelled:
            self._statsd.increment("workflow.cancellations", tags=tags)

    def on_node_start(self, node, triggers):
        return NoopSpan()

    def on_node_end(self, node, events, error, span):
        if error:
            self._statsd.increment("node.failures", tags=[f"node:{node.id}"])
        span.end()

    def on_event(self, event):
        pass

    def on_inject(self, node_specs):
        self._statsd.increment("nodes.injected", len(node_specs))

    def on_retire(self, node_id):
        self._statsd.increment("nodes.retired")
```

Use it:

```python
runner = Runner(wf, observer=DatadogObserver())
```

## Logging observer

```python
import logging
from agent_campaign.observers.base import NoopSpan

logger = logging.getLogger("workflow")

class LoggingObserver:
    def on_workflow_start(self, workflow):
        logger.info("START workflow='%s' nodes=%d", workflow.name, len(workflow.nodes))

    def on_workflow_end(self, event_count, cancelled, cycle, failed=False):
        status = "FAILED" if failed else ("CANCELLED" if cancelled else ("CYCLE" if cycle else "DONE"))
        logger.info("END status=%s events=%d", status, event_count)

    def on_node_start(self, node, triggers):
        logger.info("  NODE %s (driver=%s)", node.id, node.driver)
        return NoopSpan()

    def on_node_end(self, node, events, error, span):
        if error:
            logger.error("  ERROR %s: %s", node.id, error)
        else:
            logger.info("  DONE %s → %d events", node.id, len(events))
        span.end()

    def on_event(self, event):
        logger.debug("  EVENT %s [%s]", event.type, event.source)

    def on_inject(self, node_specs):
        logger.info("  INJECTED %d nodes", len(node_specs))

    def on_retire(self, node_id):
        logger.info("  RETIRED %s", node_id)
```

## File audit observer

```python
import json
from agent_campaign.observers.base import NoopSpan

class AuditObserver:
    def __init__(self, path: str):
        self._file = open(path, "a")

    def on_workflow_start(self, workflow):
        self._file.write(json.dumps({"event": "workflow.start", "name": workflow.name}) + "\n")
        self._file.flush()

    def on_workflow_end(self, event_count, cancelled, cycle, failed=False):
        self._file.write(json.dumps({
            "event": "workflow.end",
            "events": event_count,
            "cancelled": cancelled,
            "failed": failed,
        }) + "\n")
        self._file.flush()

    def on_node_start(self, node, triggers):
        return NoopSpan()

    def on_node_end(self, node, events, error, span):
        if error:
            self._file.write(json.dumps({
                "event": "node.failed",
                "node": node.id,
                "error": str(error),
            }) + "\n")
            self._file.flush()
        span.end()

    def on_event(self, event):
        self._file.write(json.dumps({
            "event": "event",
            "type": event.type,
            "source": event.source,
        }) + "\n")
        self._file.flush()

    def on_inject(self, node_specs):
        pass

    def on_retire(self, node_id):
        pass
```

Use it:

```python
runner = Runner(wf, observer=AuditObserver("/var/log/workflow-audit.jsonl"))
```

## See also

- [Observers](../explanation/observers.md) — the adapter concept
- [OpenTelemetry observer](../reference/otel-observer.md)
- [Use the programmatic API](programmatic-api.md)
