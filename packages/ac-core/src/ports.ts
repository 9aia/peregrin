import { Context, Data } from "effect";
import type { Effect } from "effect";
import { Event, type ExecutionContext, Node, Workflow } from "./model.js";

export class DriverError extends Data.TaggedError("DriverError")<{
  readonly reason: string;
}> {}

export interface Driver {
  readonly name: string;
  readonly invoke: (
    node: Node,
    triggers: ReadonlyArray<Event>,
    ctx: ExecutionContext,
  ) => Effect.Effect<ReadonlyArray<Event>, DriverError>;
}

export const Driver = Context.GenericTag<Driver>("ac/Driver");

export interface EventStore {
  readonly append: (event: Event) => void;
  readonly all: () => ReadonlyArray<Event>;
  readonly byType: (eventType: string) => ReadonlyArray<Event>;
  readonly seenTypes: () => ReadonlySet<string>;
  readonly latest: () => Event | null;
  readonly clear: () => void;
  readonly close: () => void;
}

export const EventStore = Context.GenericTag<EventStore>("ac/EventStore");

export interface Span {
  readonly end: () => void;
  readonly setAttribute: (key: string, value: unknown) => void;
  readonly recordError: (error: Error) => void;
  readonly getTraceparent: () => string;
}

export interface Observer {
  readonly onWorkflowStart: (workflow: Workflow) => void;
  readonly onWorkflowEnd: (
    eventCount: number,
    cancelled: boolean,
    cycle: boolean,
    failed: boolean,
  ) => void;
  readonly onNodeStart: (node: Node, triggers: ReadonlyArray<Event>) => Span;
  readonly onNodeEnd: (
    node: Node,
    events: ReadonlyArray<Event>,
    error: Error | null,
    span: Span,
  ) => void;
  readonly onEvent: (event: Event) => void;
  readonly onInject: (nodeSpecs: ReadonlyArray<Record<string, unknown>>) => void;
  readonly onRetire: (nodeId: string) => void;
}

export const Observer = Context.GenericTag<Observer>("ac/Observer");
