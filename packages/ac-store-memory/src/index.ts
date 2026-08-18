import { Event, EventStore } from "ac-core";
import { Layer } from "effect";

export class MemoryStore implements EventStore {
  private eventList: Array<Event> = [];
  private eventsByType: Record<string, Array<Event>> = {};
  private seenSet: Set<string> = new Set();

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
  }

  close(): void {}
}

export const Live: Layer.Layer<EventStore> = Layer.succeed(EventStore, new MemoryStore());
