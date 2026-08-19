import { Context, Effect, Layer } from "effect";
import { Event, Node, EventStore, Observer } from "ac-core";

function matchPattern(pattern: string, eventType: string): Array<Array<string>> | null {
  const p = pattern.split(".");
  const e = eventType.split(".");
  const step = (
    pi: number,
    ei: number,
    caps: Array<Array<string>>,
  ): Array<Array<string>> | null => {
    if (pi === p.length) return ei === e.length ? caps : null;
    const part = p[pi];
    if (part === "*") {
      if (ei >= e.length) return null;
      return step(pi + 1, ei + 1, [...caps, [e[ei]]]);
    }
    if (part === "**") {
      for (let k = 0; k <= e.length - ei; k++) {
        const r = step(pi + 1, ei + k, [...caps, e.slice(ei, ei + k)]);
        if (r !== null) return r;
      }
      return null;
    }
    if (ei >= e.length || e[ei] !== part) return null;
    return step(pi + 1, ei + 1, caps);
  };
  return step(0, 0, []);
}

export function matches(pattern: string, eventType: string): boolean {
  return matchPattern(pattern, eventType) !== null;
}

export function captureWildcards(
  pattern: string,
  eventType: string,
): Record<string, string> | null {
  const result = matchPattern(pattern, eventType);
  if (result === null) return null;
  const caps: Record<string, string> = {};
  for (let i = 0; i < result.length; i++) {
    caps[String(i + 1)] = result[i].join(".");
  }
  if (result.length > 0) caps["shard"] = result[0].join(".");
  return caps;
}

export interface PublishResult {
  readonly ready: ReadonlyArray<readonly [Node, ReadonlyArray<Event>]>;
  readonly sideEvents: ReadonlyArray<Event>;
  readonly retired: ReadonlyArray<string>;
}

export interface EventBus {
  readonly reset: (
    nodes: ReadonlyArray<Node>,
    config: Readonly<Record<string, unknown>>,
  ) => Effect.Effect<void, never, never>;
  readonly publish: (event: Event) => Effect.Effect<PublishResult, never, never>;
  readonly log: Effect.Effect<ReadonlyArray<Event>, never, never>;
  readonly fireCount: (nodeId: string) => Effect.Effect<number, never, never>;
  readonly inject: (nodes: ReadonlyArray<Record<string, unknown>>) => void;
}

export const EventBus = Context.GenericTag<EventBus>("ac/EventBus");

class EventBusLive implements EventBus {
  private nodes = new Map<string, Node>();
  private fireCounts = new Map<string, number>();
  private exhausted = new Set<string>();
  private config: Readonly<Record<string, unknown>> = {};

  constructor(
    private store: EventStore,
    private observer: Observer,
  ) {}

  readonly log = Effect.sync(() => this.store.all());

  reset(nodes: ReadonlyArray<Node>, config: Readonly<Record<string, unknown>>) {
    return Effect.sync(() => {
      this.nodes = new Map(nodes.map((node) => [node.id, node]));
      this.fireCounts.clear();
      this.exhausted.clear();
      this.config = config;
    });
  }

  publish(event: Event) {
    return Effect.sync(() => this.publishSync(event));
  }

  fireCount(nodeId: string) {
    return Effect.sync(() => this.fireCounts.get(nodeId) ?? 0);
  }

  readonly inject = (nodes: ReadonlyArray<Record<string, unknown>>): void => {
    for (const spec of nodes) {
      try {
        const node = Node.fromDict(spec);
        this.nodes.set(node.id, node);
        this.fireCounts.set(node.id, 0);
      } catch {
        // Skip invalid node specs silently.
      }
    }
    this.observer.onInject(nodes);
  };

  private publishSync(event: Event): PublishResult {
    this.store.append(event);
    this.observer.onEvent(event);
    const retired = this.checkRetirements();
    for (const id of retired) this.observer.onRetire(id);
    const ready: Array<readonly [Node, ReadonlyArray<Event>]> = [];
    const sideEvents: Array<Event> = [];
    const seen = this.store.seenTypes();
    for (const node of this.nodes.values()) {
      if (!this.isEligible(node, seen)) {
        if (
          this.isMaxFiresExhausted(node) &&
          !this.isUntilBlocked(node, seen) &&
          !this.exhausted.has(node.id) &&
          this.matchesAny(node, event)
        ) {
          if (node.mode !== "all" || this.allPatternsSatisfied(node, seen)) {
            this.exhausted.add(node.id);
            sideEvents.push(Event.nodeExhausted(node.id, node.maxFires));
          }
        }
        continue;
      }
      if (!this.matchesAny(node, event)) continue;
      let triggers: ReadonlyArray<Event>;
      if (node.mode === "all") {
        if (!this.allPatternsSatisfied(node, seen)) continue;
        triggers = this.collectAllTriggers(node, seen);
      } else if (node.fanIn) {
        triggers = this.collectFanInTriggers(node, seen);
      } else {
        triggers = [event];
      }
      ready.push([node, triggers]);
      this.fireCounts.set(node.id, (this.fireCounts.get(node.id) ?? 0) + 1);
    }
    return { ready, sideEvents, retired };
  }

  private checkRetirements(): Array<string> {
    const seen = this.store.seenTypes();
    const toRetire: Array<string> = [];
    for (const [id, node] of this.nodes) {
      if (!node.retireOn) continue;
      for (const retireType of node.retireOn) {
        if (seen.has(retireType)) {
          toRetire.push(id);
          break;
        }
      }
    }
    for (const id of toRetire) this.nodes.delete(id);
    return toRetire;
  }

  private isMaxFiresExhausted(node: Node): boolean {
    return node.maxFires > 0 && (this.fireCounts.get(node.id) ?? 0) >= node.maxFires;
  }

  private isUntilBlocked(node: Node, seen: ReadonlySet<string>): boolean {
    return node.until.some((untilType) => seen.has(untilType));
  }

  private isEligible(node: Node, seen: ReadonlySet<string>): boolean {
    return (
      !this.isMaxFiresExhausted(node) &&
      !this.isUntilBlocked(node, seen) &&
      this.isFanInSatisfied(node, seen)
    );
  }

  private isFanInSatisfied(node: Node, seen: ReadonlySet<string>): boolean {
    if (!node.fanIn) return true;
    const count = this.resolveFanInCount(node);
    if (count === undefined) return true;
    const pattern = node.on[0] ?? "";
    if (!pattern) return false;
    return this.countMatching(pattern, seen) >= count;
  }

  private resolveFanInCount(node: Node): number | undefined {
    if (!node.fanIn) return undefined;
    if ("count" in node.fanIn) {
      const c = Number(node.fanIn.count);
      return Number.isNaN(c) ? undefined : c;
    }
    if ("count_from" in node.fanIn) {
      const path = String(node.fanIn.count_from);
      const parts = path.split(".");
      let current: unknown = this.config;
      for (const part of parts) {
        if (
          current &&
          typeof current === "object" &&
          part in (current as Record<string, unknown>)
        ) {
          current = (current as Record<string, unknown>)[part];
        } else {
          return undefined;
        }
      }
      if (Array.isArray(current)) return current.length;
      const n = Number(current);
      return Number.isNaN(n) ? undefined : n;
    }
    return undefined;
  }

  private countMatching(pattern: string, seen: ReadonlySet<string>): number {
    if (!pattern.includes("*")) {
      return this.store.byType(pattern).length;
    }
    let total = 0;
    for (const type of seen) {
      if (matches(pattern, type)) {
        total += this.store.byType(type).length;
      }
    }
    return total;
  }

  private eventsMatching(pattern: string, seen: ReadonlySet<string>): ReadonlyArray<Event> {
    if (!pattern.includes("*")) {
      return this.store.byType(pattern);
    }
    const result: Array<Event> = [];
    for (const type of seen) {
      if (matches(pattern, type)) {
        result.push(...this.store.byType(type));
      }
    }
    return result;
  }

  private collectFanInTriggers(node: Node, seen: ReadonlySet<string>): ReadonlyArray<Event> {
    const pattern = node.on[0] ?? "";
    if (!pattern) return [];
    return this.eventsMatching(pattern, seen);
  }

  private matchesAny(node: Node, event: Event): boolean {
    return node.on.some((pattern) => matches(pattern, event.type));
  }

  private allPatternsSatisfied(node: Node, seen: ReadonlySet<string>): boolean {
    return node.on.every((pattern) => {
      if (pattern.includes("*")) {
        for (const type of seen) {
          if (matches(pattern, type)) return true;
        }
        return false;
      }
      return seen.has(pattern);
    });
  }

  private firstMatching(pattern: string, seen: ReadonlySet<string>): Event | null {
    if (!pattern.includes("*")) {
      const events = this.store.byType(pattern);
      return events[0] ?? null;
    }
    for (const type of seen) {
      if (matches(pattern, type)) {
        const events = this.store.byType(type);
        if (events.length > 0) return events[0];
      }
    }
    return null;
  }

  private collectAllTriggers(node: Node, seen: ReadonlySet<string>): ReadonlyArray<Event> {
    const triggers: Array<Event> = [];
    const seenPatterns = new Set<string>();
    for (const pattern of node.on) {
      if (seenPatterns.has(pattern)) continue;
      seenPatterns.add(pattern);
      const first = this.firstMatching(pattern, seen);
      if (first) triggers.push(first);
    }
    return triggers;
  }
}

export const EventBusLiveLayer = Layer.effect(
  EventBus,
  Effect.gen(function* () {
    const store = yield* EventStore;
    const observer = yield* Observer;
    return new EventBusLive(store, observer);
  }),
);
