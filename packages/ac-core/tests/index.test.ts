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
});
