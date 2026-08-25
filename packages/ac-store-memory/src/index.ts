import { Event, EventStore, StateStore } from "handl-core";
import { Layer } from "effect";

export class MemoryStore implements EventStore {
  private eventList: Array<Event> = [];
  private eventsByType: Record<string, Array<Event>> = {};
  private seenSet: Set<string> = new Set();
  private stateMap = new Map<string, unknown>();

  append(event: Event): void {
    this.eventList.push(event);
    this.seenSet.add(event.type);
    const list = (this.eventsByType[event.type] ??= []);
    list.push(event);
  }

  all(): ReadonlyArray<Event> {
    return this.eventList;
  }

  byType(eventType: string): ReadonlyArray<Event> {
    return this.eventsByType[eventType] ?? [];
  }

  seenTypes(): ReadonlySet<string> {
    return new Set(this.seenSet);
  }

  latest(): Event | null {
    return this.eventList.at(-1) ?? null;
  }

  clear(): void {
    this.eventList = [];
    this.eventsByType = {};
    this.seenSet = new Set();
    this.stateMap.clear();
  }

  close(): void {}

  get(key: string): unknown {
    return this.stateMap.get(key);
  }

  set(key: string, value: unknown): void {
    this.stateMap.set(key, value);
  }

  toRecord(): Readonly<Record<string, unknown>> {
    return Object.fromEntries(this.stateMap.entries()) as Readonly<Record<string, unknown>>;
  }
}

const liveStore = new MemoryStore();

export const EventStoreLive: Layer.Layer<EventStore> = Layer.succeed(EventStore, liveStore);
export const StateStoreLive: Layer.Layer<StateStore> = Layer.succeed(StateStore, liveStore);
export const Live: Layer.Layer<EventStore | StateStore> = Layer.merge(
  EventStoreLive,
  StateStoreLive,
);
