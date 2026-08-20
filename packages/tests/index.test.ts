import { describe, it, expect } from "vite-plus/test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { Effect } from "effect";
import { DriverError, Event, TemplateError } from "ac-core";
import type { Driver, ExecutionContext, Node } from "ac-core";
import { captureWildcards, matches } from "ac-engine";
import { loadWorkflow } from "ac-loader-yaml";
import { resolveTemplate } from "ac-template-engine";
import { runWorkflow } from "ac-testing-library";

const mkTmp = () => fs.mkdtempSync(path.join(os.tmpdir(), "ac-integration-"));

const writeFiles = (dir: string, files: Record<string, string>) => {
  for (const [relative, content] of Object.entries(files)) {
    const full = path.join(dir, relative);
    fs.mkdirSync(path.dirname(full), { recursive: true });
    fs.writeFileSync(full, content, "utf8");
  }
};

const pythonDriver: Driver = {
  name: "python",
  invoke(node: Node) {
    const raw = node.with_.ids;
    const ids = Array.isArray(raw) ? (raw as ReadonlyArray<string>) : ["a", "b", "c"];
    const template = node.emits[0] ?? `${node.id}.done`;
    return Effect.succeed(ids.map((id) => new Event(template.replace("*", id), { id }, node.id)));
  },
};

const shellDriver: Driver = {
  name: "shell",
  invoke(node: Node, triggers: ReadonlyArray<Event>, ctx: ExecutionContext) {
    return Effect.gen(function* () {
      const raw = typeof node.with_.cmd === "string" ? node.with_.cmd : "";
      const cmd = yield* resolveTemplate(raw, triggers, ctx, node).pipe(
        Effect.mapError((error: TemplateError) => new DriverError({ reason: error.reason })),
      );
      const trigger = triggers[0];
      const pattern = node.on.find((p) => trigger && matches(p, trigger.type)) ?? node.on[0] ?? "";
      const caps = trigger ? captureWildcards(pattern, trigger.type) : null;
      const shard = caps?.["1"] ?? "";
      const template = node.emits[0] ?? `${node.id}.done`;
      const type = template.includes("*") ? template.replace("*", shard) : template;
      return [new Event(type, { stdout: cmd }, node.id)];
    });
  },
};

const looperDriver: Driver = {
  name: "looper",
  invoke(node: Node, _triggers: ReadonlyArray<Event>, ctx: ExecutionContext) {
    if (ctx.fires >= 2) {
      return Effect.succeed([new Event("loop.success", { fires: ctx.fires }, node.id)]);
    }
    return Effect.succeed([new Event("loop.retry", { fires: ctx.fires }, node.id)]);
  },
};

const drivers: ReadonlyArray<Driver> = [pythonDriver, shellDriver, looperDriver];

const runFile = async (dir: string, file = "workflow.yaml") => {
  const workflow = await Effect.runPromise(loadWorkflow(path.join(dir, file)));
  return Effect.runPromise(runWorkflow(workflow, drivers));
};

describe("@ac/tests integration", () => {
  it("discovers snippets and included files through the file router", async () => {
    const dir = mkTmp();
    try {
      writeFiles(dir, {
        ".ac/fragments.yaml": `
          snippets:
            greet:
              driver: shell
              with:
                cmd: "echo hello \${name}"
        `,
        "workflow.yaml": `
          name: snippets
          include: [.ac/fragments.yaml]
          nodes:
            - id: greet
              snippet: greet
              on: [start]
              emits: [greet.done]
        `,
      });
      const log = await runFile(dir);
      expect(log.some((event) => event.type === "greet.done")).toBe(true);
      const greet = log.find((event) => event.type === "greet.done");
      expect(greet?.payload.stdout).toContain("hello ${name}");
      expect(log.some((event) => event.type === "capability.violation")).toBe(false);
      expect(log.some((event) => event.type === "workflow.done")).toBe(true);
    } finally {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });

  it("runs a map-reduce workflow with fan_in and wildcard triggers", async () => {
    const dir = mkTmp();
    try {
      writeFiles(dir, {
        ".ac/map.node.yml": `
          id: map
          driver: python
          on: [start]
          emits: [shard.*.ready]
          max_fires: 0
          with:
            ids: ["1", "2"]
        `,
        "workflow.yaml": `
          name: map-reduce
          nodes:
            - id: fanin
              on: [shard.*.ready]
              fan_in:
                count: 2
                emit: reduce.done
              max_fires: 1
            - id: reduce
              driver: shell
              on: [reduce.done]
              emits: [reduce.complete]
              with:
                cmd: "echo reduced"
        `,
      });
      const log = await runFile(dir);
      const types = log.map((event) => event.type);
      expect(types).toContain("shard.1.ready");
      expect(types).toContain("shard.2.ready");
      expect(types).toContain("reduce.done");
      expect(types).toContain("reduce.complete");
      expect(types).toContain("workflow.done");
      expect(types).not.toContain("workflow.failed");
    } finally {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });

  it("forwards wildcard events while preserving payload", async () => {
    const dir = mkTmp();
    try {
      writeFiles(dir, {
        ".ac/map.node.yml": `
          id: map
          driver: python
          on: [start]
          emits: [shard.*.ready]
          max_fires: 0
          with:
            ids: ["1", "2"]
        `,
        "workflow.yaml": `
          name: forward-wildcard
          nodes:
            - id: bridge
              on: [shard.*.ready]
              forward: [proc.*.run]
              max_fires: 0
            - id: sink
              driver: shell
              on: [proc.*.run]
              emits: [sink.*.done]
              max_fires: 0
              with:
                cmd: "echo done"
        `,
      });
      const log = await runFile(dir);
      const types = log.map((event) => event.type);
      expect(types).toContain("proc.1.run");
      expect(types).toContain("proc.2.run");
      expect(types).toContain("sink.1.done");
      expect(types).toContain("sink.2.done");
      const sink1 = log.find((event) => event.type === "sink.1.done");
      const sink2 = log.find((event) => event.type === "sink.2.done");
      expect(sink1).toBeDefined();
      expect(sink2).toBeDefined();
      expect(sink1?.payload.stdout).toBe("echo done");
      expect(sink2?.payload.stdout).toBe("echo done");
      const proc1 = log.find((event) => event.type === "proc.1.run");
      const proc2 = log.find((event) => event.type === "proc.2.run");
      expect(proc1?.payload.id).toBe("1");
      expect(proc2?.payload.id).toBe("2");
    } finally {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });

  it("enforces capabilities even in dry runs", async () => {
    const dir = mkTmp();
    try {
      writeFiles(dir, {
        "workflow.yaml": `
          name: capabilities
          capabilities:
            default_policy: deny
            shell:
              allow: ["echo *"]
          nodes:
            - id: safe
              driver: shell
              on: [start]
              emits: [safe.done]
              with:
                cmd: "echo hello"
            - id: danger
              driver: shell
              on: [safe.done]
              emits: [danger.done]
              with:
                cmd: "rm -rf /"
        `,
      });
      const workflow = await Effect.runPromise(loadWorkflow(path.join(dir, "workflow.yaml")));
      const log = await Effect.runPromise(runWorkflow(workflow, drivers));
      const types = log.map((event) => event.type);
      expect(types).toContain("safe.done");
      expect(types).toContain("capability.violation");
      expect(types).not.toContain("danger.done");
      expect(types).toContain("workflow.done");
    } finally {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });

  it("honors max_fires, until, and retire_on on a looping node", async () => {
    const dir = mkTmp();
    try {
      writeFiles(dir, {
        "workflow.yaml": `
          name: loop
          nodes:
            - id: loop
              driver: looper
              on: [start, loop.retry]
              emits: [loop.success, loop.retry]
              max_fires: 3
              until: [loop.success]
              retire_on: [loop.success]
            - id: after
              driver: shell
              on: [loop.success]
              emits: [after.done]
              with:
                cmd: "echo done"
        `,
      });
      const log = await runFile(dir);
      const types = log.map((event) => event.type);
      expect(types).toContain("loop.retry");
      expect(types).toContain("loop.success");
      expect(types).toContain("after.done");
      expect(types).toContain("workflow.done");
      const retries = log.filter((event) => event.type === "loop.retry");
      expect(retries.length).toBe(1);
    } finally {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });
});
