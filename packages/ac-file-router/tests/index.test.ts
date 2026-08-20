import { describe, it, expect } from "vite-plus/test";
import { Effect } from "effect";
import { discover } from "../src/index.ts";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

describe("ac-file-router", () => {
  it("discovers .node.yml files", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-file-router-"));
    try {
      fs.writeFileSync(path.join(tmp, "demo.node.yml"), "");
      const result = await Effect.runPromise(discover(tmp));
      expect(result.nodes).toHaveLength(1);
      expect(result.nodes[0].name).toBe("demo");
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("discovers all recognized file kinds", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-file-router-"));
    try {
      fs.writeFileSync(path.join(tmp, "alpha.node.yml"), "");
      fs.writeFileSync(path.join(tmp, "beta.snippet.yml"), "");
      fs.writeFileSync(path.join(tmp, "gamma.event.json"), "{}");
      fs.writeFileSync(path.join(tmp, "delta.handler.py"), "");
      fs.writeFileSync(path.join(tmp, "epsilon.config.yml"), "");
      fs.writeFileSync(path.join(tmp, "zeta.secret.yml"), "");
      fs.writeFileSync(path.join(tmp, ".env"), "");
      const result = await Effect.runPromise(discover(tmp));
      expect(result.nodes.map((e) => e.name)).toEqual(["alpha"]);
      expect(result.snippets.map((e) => e.name)).toEqual(["beta"]);
      expect(result.events.map((e) => e.name)).toEqual(["gamma"]);
      expect(result.handlers.map((e) => e.name)).toEqual(["delta"]);
      expect(result.secrets.map((e) => e.name)).toEqual(["zeta"]);
      expect(result.config).toHaveLength(1);
      expect(result.env).toHaveLength(1);
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("discovers nested files", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-file-router-"));
    try {
      fs.mkdirSync(path.join(tmp, "sub"));
      fs.writeFileSync(path.join(tmp, "sub", "nested.node.yml"), "");
      const result = await Effect.runPromise(discover(tmp));
      expect(result.nodes).toHaveLength(1);
      expect(result.nodes[0].name).toBe("nested");
      expect(result.nodes[0].path).toContain("sub");
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("prioritizes the .ac directory over the root", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-file-router-"));
    try {
      fs.mkdirSync(path.join(tmp, ".ac"));
      fs.writeFileSync(path.join(tmp, "ghost.node.yml"), "id: root\n");
      fs.writeFileSync(path.join(tmp, ".ac", "ghost.node.yml"), "id: ac\n");
      const result = await Effect.runPromise(discover(tmp));
      expect(result.nodes).toHaveLength(1);
      expect(result.nodes[0].path).toContain(".ac");
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("skips hidden directories except .ac", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-file-router-"));
    try {
      fs.mkdirSync(path.join(tmp, ".ac"));
      fs.writeFileSync(path.join(tmp, ".ac", "visible.node.yml"), "");
      fs.mkdirSync(path.join(tmp, ".git"));
      fs.writeFileSync(path.join(tmp, ".git", "ignored.node.yml"), "");
      const result = await Effect.runPromise(discover(tmp));
      expect(result.nodes).toHaveLength(1);
      expect(result.nodes[0].name).toBe("visible");
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("sorts config files", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ac-file-router-"));
    try {
      fs.writeFileSync(path.join(tmp, "z.config.yml"), "");
      fs.writeFileSync(path.join(tmp, "a.config.yml"), "");
      const result = await Effect.runPromise(discover(tmp));
      expect(result.config.map((p) => path.basename(p))).toEqual(["a.config.yml", "z.config.yml"]);
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("fails for a missing base directory", async () => {
    await expect(
      Effect.runPromise(discover("/nonexistent/path/should/fail")),
    ).rejects.toBeDefined();
  });
});
