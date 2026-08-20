import { describe, it, expect } from "vite-plus/test";
import { Event, EventSchema, Node, Workflow } from "../src/index.ts";

describe("ac-core", () => {
  it("creates an Event with a type, payload and source", () => {
    const event = new Event("test", { value: 1 }, "src");
    expect(event.type).toBe("test");
    expect(event.payload).toEqual({ value: 1 });
    expect(event.source).toBe("src");
  });

  it("compiles an EventSchema from fields", () => {
    const schema = new EventSchema("user", { name: "string", age: "integer?" });
    const compiled = schema.compiledSchema();
    expect(compiled).not.toBeNull();
    expect(compiled!.properties).toEqual({
      name: { type: "string" },
      age: { type: "integer" },
    });
    expect(compiled!.required).toEqual(["name"]);
  });

  it("compiles an unknown field type as string", () => {
    const schema = new EventSchema("custom", { value: "custom-type" });
    const compiled = schema.compiledSchema()!;
    expect(compiled.properties).toEqual({ value: { type: "string" } });
  });

  it("creates a Node from a dictionary", () => {
    const node = Node.fromDict({ id: "a", driver: "noop" });
    expect(node.id).toBe("a");
    expect(node.driver).toBe("noop");
  });

  it("creates a Workflow from a dictionary", () => {
    const workflow = Workflow.fromDict({ name: "demo", nodes: [] });
    expect(workflow.name).toBe("demo");
    expect(workflow.nodes).toHaveLength(0);
  });

  it("stores annotations on EventSchema constructor", () => {
    const schema = new EventSchema("x", { name: "string" }, {}, { title: "User" });
    expect(schema.annotations).toEqual({ title: "User" });
  });

  it("parses annotations from EventSchema.fromDict", () => {
    const schema = EventSchema.fromDict({
      type: "x",
      fields: { name: "string" },
      annotations: { title: "User" },
    });
    expect(schema.annotations).toEqual({ title: "User" });
  });

  it("defaults EventSchema annotations to {}", () => {
    const schema = EventSchema.fromDict({ type: "x" });
    expect(schema.annotations).toEqual({});
  });

  it("parses annotations from Node.fromDict", () => {
    const node = Node.fromDict({ id: "a", driver: "noop", annotations: { title: "A" } });
    expect(node.annotations).toEqual({ title: "A" });
  });

  it("stores annotations on Node constructor", () => {
    const node = new Node(
      "a",
      "noop",
      "",
      [],
      [],
      "any",
      {},
      1,
      [],
      [],
      {},
      {},
      null,
      null,
      [],
      {},
      {},
      {},
      { title: "A" },
    );
    expect(node.annotations).toEqual({ title: "A" });
  });

  it("parses input and output on a node", () => {
    const node = Node.fromDict({
      id: "a",
      driver: "shell",
      input: {
        fields: { cmd: "string" },
      },
      output: {
        "cmd.done": { fields: { stdout: "string" } },
      },
    });
    expect(node.input?.fields).toEqual({ cmd: "string" });
    expect(node.output).toHaveLength(1);
    expect(node.output[0].type).toBe("cmd.done");
  });

  it("includes input and output in Node.toDict", () => {
    const node = new Node(
      "a",
      "shell",
      "",
      [],
      [],
      "any",
      {},
      1,
      [],
      [],
      {},
      {},
      null,
      null,
      [],
      {},
      {},
      {},
      {},
      new EventSchema("", { cmd: "string" }),
      [new EventSchema("cmd.done", { stdout: "string" })],
    );
    const d = node.toDict();
    expect(d).toHaveProperty("input", { fields: { cmd: "string" } });
    expect(d).toHaveProperty("output", { "cmd.done": { fields: { stdout: "string" } } });
  });

  it("includes annotations in Node.toDict when non-empty and omits when empty", () => {
    const withAnnotations = new Node(
      "a",
      "noop",
      "",
      [],
      [],
      "any",
      {},
      1,
      [],
      [],
      {},
      {},
      null,
      null,
      [],
      {},
      {},
      {},
      { title: "A" },
    );
    expect(withAnnotations.toDict()).toHaveProperty("annotations", { title: "A" });
    const withoutAnnotations = new Node("b", "noop");
    expect(withoutAnnotations.toDict()).not.toHaveProperty("annotations");
  });

  it("parses workflow annotations", () => {
    const workflow = Workflow.fromDict({ name: "demo", nodes: [], annotations: { title: "Demo" } });
    expect(workflow.annotations).toEqual({ title: "Demo" });
  });

  it("passes through event and node annotations in Workflow.fromDict", () => {
    const workflow = Workflow.fromDict({
      name: "demo",
      events: [{ type: "x", fields: {}, annotations: { title: "X" } }],
      nodes: [{ id: "a", driver: "noop", annotations: { title: "A" } }],
    });
    expect(workflow.events[0].annotations).toEqual({ title: "X" });
    expect(workflow.nodes[0].annotations).toEqual({ title: "A" });
  });

  it("includes annotations in Workflow.toDict when non-empty and omits when empty", () => {
    const nonEmpty = Workflow.fromDict({
      name: "demo",
      events: [{ type: "x", fields: {}, annotations: { title: "X" } }],
      nodes: [{ id: "a", driver: "noop", annotations: { title: "A" } }],
      annotations: { title: "Demo" },
    });
    const d = nonEmpty.toDict();
    expect(d).toHaveProperty("annotations", { title: "Demo" });
    expect((d.events as Array<Record<string, unknown>>)[0]).toHaveProperty("annotations", {
      title: "X",
    });
    expect((d.nodes as Array<Record<string, unknown>>)[0]).toHaveProperty("annotations", {
      title: "A",
    });

    const empty = Workflow.fromDict({
      name: "empty",
      events: [{ type: "x" }],
      nodes: [{ id: "a", driver: "noop" }],
    });
    const e = empty.toDict();
    expect(e).not.toHaveProperty("annotations");
    expect((e.events as Array<Record<string, unknown>>)[0]).not.toHaveProperty("annotations");
    expect((e.nodes as Array<Record<string, unknown>>)[0]).not.toHaveProperty("annotations");
  });
});
