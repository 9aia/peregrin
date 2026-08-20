import { describe, it, expect } from "vite-plus/test";
import { Effect } from "effect";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { loadWorkflowFromText, loadRaw } from "../src/index.ts";

describe("ac-loader-yaml", () => {
  it("loads a minimal workflow from text", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-loader-yaml-"));
    try {
      const wf = await Effect.runPromise(loadWorkflowFromText("name: test\nnodes: []", tmp));
      expect(wf.name).toBe("test");
      expect(wf.nodes).toHaveLength(0);
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("fails when nodes is not an array", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-loader-yaml-"));
    try {
      await expect(
        Effect.runPromise(loadWorkflowFromText("name: test\nnodes: notArray", tmp)),
      ).rejects.toBeDefined();
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("merges include into node with and local wins", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-loader-yaml-"));
    try {
      fs.writeFileSync(path.join(tmp, "env.yaml"), "env:\n  REGION: us-east-1\ntimeout: 300\n");
      const wf = await Effect.runPromise(
        loadWorkflowFromText(
          [
            "name: test",
            "nodes:",
            "  - id: deploy",
            "    driver: shell",
            "    on: [start]",
            "    emits: [deploy.done]",
            "    with:",
            "      include: env.yaml",
            "      cmd: deploy.sh",
          ].join("\n"),
          tmp,
        ),
      );
      const with_ = wf.nodes[0].with_;
      expect(with_.env).toEqual({ REGION: "us-east-1" });
      expect(with_.timeout).toBe(300);
      expect(with_.cmd).toBe("deploy.sh");
      expect(with_.include).toBeUndefined();
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("injects a whole dict via include expression", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-loader-yaml-"));
    try {
      fs.writeFileSync(path.join(tmp, "env.yaml"), "REGION: us-east-1\nCLUSTER: prod\n");
      const wf = await Effect.runPromise(
        loadWorkflowFromText(
          [
            "name: test",
            "nodes:",
            "  - id: deploy",
            "    driver: shell",
            "    on: [start]",
            "    with:",
            "      env: ${include(env.yaml)}",
            "      cmd: deploy.sh",
          ].join("\n"),
          tmp,
        ),
      );
      expect(wf.nodes[0].with_.env).toEqual({ REGION: "us-east-1", CLUSTER: "prod" });
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("injects a whole list via include expression", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-loader-yaml-"));
    try {
      fs.writeFileSync(
        path.join(tmp, "shards.yaml"),
        "- id: us\n  driver: shell\n  on: [start]\n  emits: [us.done]\n- id: eu\n  driver: shell\n  on: [start]\n  emits: [eu.done]\n",
      );
      const wf = await Effect.runPromise(
        loadWorkflowFromText(["name: test", "nodes: ${include(shards.yaml)}"].join("\n"), tmp),
      );
      expect(wf.nodes).toHaveLength(2);
      expect(wf.nodes[0].id).toBe("us");
      expect(wf.nodes[1].id).toBe("eu");
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("injects scalar and partial include expressions", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-loader-yaml-"));
    try {
      fs.writeFileSync(path.join(tmp, "version.yaml"), "1.2.3");
      const wf = await Effect.runPromise(
        loadWorkflowFromText(
          [
            "name: deploy-${include(version.yaml)}",
            "nodes:",
            "  - id: deploy",
            "    driver: shell",
            "    on: [start]",
            "    with:",
            "      cmd: echo version ${include(version.yaml)}",
          ].join("\n"),
          tmp,
        ),
      );
      expect(wf.name).toBe("deploy-1.2.3");
      expect(wf.nodes[0].with_.cmd).toBe("echo version 1.2.3");
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("uses first include when include is a list", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-loader-yaml-"));
    try {
      fs.writeFileSync(
        path.join(tmp, "a.yaml"),
        "id: node-a\ndriver: shell\non: [start]\nemits: [a.done]\n",
      );
      fs.writeFileSync(path.join(tmp, "b.yaml"), "id: node-b\non: [a.done]\nemits: [b.done]\n");
      const wf = await Effect.runPromise(
        loadWorkflowFromText(
          ["name: test", "nodes:", "  - include: [a.yaml, b.yaml]"].join("\n"),
          tmp,
        ),
      );
      expect(wf.nodes).toHaveLength(1);
      expect(wf.nodes[0].id).toBe("node-a");
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("recursively merges include files", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-loader-yaml-"));
    try {
      fs.writeFileSync(path.join(tmp, "base.yaml"), "env:\n  PYTHONPATH: /app\n");
      fs.writeFileSync(
        path.join(tmp, "prod.yaml"),
        "include: base.yaml\nenv:\n  REGION: us-east-1\n",
      );
      const wf = await Effect.runPromise(
        loadWorkflowFromText(
          [
            "name: test",
            "nodes:",
            "  - id: deploy",
            "    driver: shell",
            "    on: [start]",
            "    with:",
            "      include: prod.yaml",
          ].join("\n"),
          tmp,
        ),
      );
      expect(wf.nodes[0].with_.env).toEqual({
        PYTHONPATH: "/app",
        REGION: "us-east-1",
      });
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("resolves recursive include expressions", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-loader-yaml-"));
    try {
      fs.writeFileSync(path.join(tmp, "base.yaml"), "PYTHONPATH: /app\n");
      fs.writeFileSync(path.join(tmp, "prod.yaml"), "include: base.yaml\nREGION: us-east-1\n");
      const wf = await Effect.runPromise(
        loadWorkflowFromText(
          [
            "name: test",
            "nodes:",
            "  - id: deploy",
            "    driver: shell",
            "    on: [start]",
            "    with:",
            "      env: ${include(prod.yaml)}",
          ].join("\n"),
          tmp,
        ),
      );
      const env = wf.nodes[0].with_.env as Record<string, unknown>;
      expect(env.PYTHONPATH).toBe("/app");
      expect(env.REGION).toBe("us-east-1");
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("detects circular includes by key", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-loader-yaml-"));
    try {
      fs.writeFileSync(path.join(tmp, "a.yaml"), "include: b.yaml\nvalue: a\n");
      fs.writeFileSync(path.join(tmp, "b.yaml"), "include: a.yaml\nvalue: b\n");
      await expect(Effect.runPromise(loadRaw("include: a.yaml", tmp))).rejects.toBeDefined();
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("detects circular includes by expression", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-loader-yaml-"));
    try {
      fs.writeFileSync(path.join(tmp, "a.yaml"), "value: ${include(b.yaml)}\n");
      fs.writeFileSync(path.join(tmp, "b.yaml"), "value: ${include(a.yaml)}\n");
      await expect(
        Effect.runPromise(
          loadRaw(
            ["nodes:", "  - id: x", "    with:", "      val: ${include(a.yaml)}"].join("\n"),
            tmp,
          ),
        ),
      ).rejects.toBeDefined();
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("detects self includes", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-loader-yaml-"));
    try {
      fs.writeFileSync(path.join(tmp, "main.yaml"), "include: main.yaml\n");
      await expect(Effect.runPromise(loadRaw("include: main.yaml", tmp))).rejects.toBeDefined();
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("raises for missing include files", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-loader-yaml-"));
    try {
      fs.writeFileSync(
        path.join(tmp, "main.yaml"),
        "nodes:\n  - id: x\n    with:\n      include: missing.yaml\n",
      );
      await expect(
        Effect.runPromise(
          loadWorkflowFromText("nodes:\n  - id: x\n    with:\n      include: missing.yaml", tmp),
        ),
      ).rejects.toBeDefined();
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("merges a snippet into a node", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-loader-yaml-"));
    try {
      const wf = await Effect.runPromise(
        loadWorkflowFromText(
          [
            "name: test",
            "snippets:",
            "  base:",
            "    driver: shell",
            "    emits: [base.done]",
            "    with:",
            "      timeout: 600",
            "      env:",
            "        A: 1",
            "nodes:",
            "  - id: n",
            "    snippet: base",
            "    on: [start]",
            "    with:",
            "      env:",
            "        B: 2",
          ].join("\n"),
          tmp,
        ),
      );
      const node = wf.nodes[0];
      expect(node.driver).toBe("shell");
      expect(node.emits).toEqual(["base.done"]);
      expect(node.with_.timeout).toBe(600);
      expect(node.with_.env).toEqual({ A: 1, B: 2 });
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("injects a scalar snippet via expression", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-loader-yaml-"));
    try {
      const wf = await Effect.runPromise(
        loadWorkflowFromText(
          [
            "name: test",
            "snippets:",
            "  cmd: echo hello",
            "nodes:",
            "  - id: n",
            "    driver: shell",
            "    on: [start]",
            "    with:",
            "      cmd: ${snippet(cmd)}",
          ].join("\n"),
          tmp,
        ),
      );
      expect(wf.nodes[0].with_.cmd).toBe("echo hello");
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("injects a list snippet via expression", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-loader-yaml-"));
    try {
      const wf = await Effect.runPromise(
        loadWorkflowFromText(
          [
            "name: test",
            "snippets:",
            "  triggers: [pr.opened, pr.updated]",
            "nodes:",
            "  - id: n",
            "    driver: shell",
            "    on: ${snippet(triggers)}",
            "    with:",
            "      cmd: echo ok",
          ].join("\n"),
          tmp,
        ),
      );
      expect(wf.nodes[0].on).toEqual(["pr.opened", "pr.updated"]);
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("injects a dict snippet via expression", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-loader-yaml-"));
    try {
      const wf = await Effect.runPromise(
        loadWorkflowFromText(
          [
            "name: test",
            "snippets:",
            "  env:",
            "    REGION: us-east-1",
            "    CLUSTER: prod",
            "nodes:",
            "  - id: n",
            "    driver: shell",
            "    on: [start]",
            "    with:",
            "      env: ${snippet(env)}",
            "      cmd: echo ok",
          ].join("\n"),
          tmp,
        ),
      );
      expect(wf.nodes[0].with_.env).toEqual({
        REGION: "us-east-1",
        CLUSTER: "prod",
      });
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("interpolates a snippet into a partial string", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-loader-yaml-"));
    try {
      const wf = await Effect.runPromise(
        loadWorkflowFromText(
          [
            "name: test",
            "snippets:",
            "  model: sonnet",
            "nodes:",
            "  - id: n",
            "    driver: shell",
            "    on: [start]",
            "    with:",
            "      cmd: run --model ${snippet(model)}",
          ].join("\n"),
          tmp,
        ),
      );
      expect(wf.nodes[0].with_.cmd).toBe("run --model sonnet");
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("raises for unknown snippet references", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-loader-yaml-"));
    try {
      await expect(
        Effect.runPromise(
          loadWorkflowFromText(
            ["name: test", "nodes:", "  - id: n", "    snippet: missing", "    on: [start]"].join(
              "\n",
            ),
            tmp,
          ),
        ),
      ).rejects.toBeDefined();
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("resolves three levels of snippet inheritance", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-loader-yaml-"));
    try {
      const wf = await Effect.runPromise(
        loadWorkflowFromText(
          [
            "name: test",
            "snippets:",
            "  a:",
            "    with:",
            "      x: 1",
            "  b:",
            "    snippet: a",
            "    with:",
            "      y: 2",
            "  c:",
            "    snippet: b",
            "    with:",
            "      z: 3",
            "nodes:",
            "  - id: n",
            "    snippet: c",
            "    on: [start]",
          ].join("\n"),
          tmp,
        ),
      );
      const with_ = wf.nodes[0].with_;
      expect(with_.x).toBe(1);
      expect(with_.y).toBe(2);
      expect(with_.z).toBe(3);
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("detects self-referencing snippet inheritance", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-loader-yaml-"));
    try {
      await expect(
        Effect.runPromise(
          loadWorkflowFromText(
            [
              "name: test",
              "snippets:",
              "  loop:",
              "    snippet: loop",
              "nodes:",
              "  - id: n",
              "    snippet: loop",
              "    on: [start]",
            ].join("\n"),
            tmp,
          ),
        ),
      ).rejects.toBeDefined();
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("auto-discovers nodes and snippets from the .ac directory", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-loader-yaml-"));
    try {
      const acDir = path.join(tmp, ".ac");
      fs.mkdirSync(acDir);
      fs.writeFileSync(
        path.join(acDir, "shared.snippet.yml"),
        ["driver: shell", "with:", "  timeout: 1200"].join("\n"),
      );
      fs.writeFileSync(
        path.join(acDir, "auto.node.yml"),
        ["id: auto", "driver: shell", "on: [start]"].join("\n"),
      );
      const wf = await Effect.runPromise(
        loadWorkflowFromText(
          ["name: test", "nodes:", "  - id: n", "    snippet: shared", "    on: [start]"].join(
            "\n",
          ),
          tmp,
        ),
      );
      expect(wf.nodes).toHaveLength(2);
      expect(wf.nodes[0].id).toBe("auto");
      expect(wf.nodes[1].id).toBe("n");
      expect(wf.nodes[1].with_.timeout).toBe(1200);
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });
});
