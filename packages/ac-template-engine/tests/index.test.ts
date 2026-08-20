import { describe, it, expect } from "vite-plus/test";
import { Effect } from "effect";
import { resolveTemplate, resolveValue, resolveValueForNode } from "../src/index.ts";
import { Event, Node, type ExecutionContext } from "ac-core";

const makeCtx = (
  config: Record<string, unknown> = {},
  secrets: Record<string, string> = {},
  state: Record<string, unknown> = {},
  fires = 0,
): ExecutionContext => ({
  config,
  secrets,
  log: [],
  emit: () => {},
  inject: () => {},
  dryRun: false,
  idempotencyKey: "1",
  traceparent: "",
  state: {
    get: () => undefined,
    set: () => {},
    clear: () => undefined,
    close: () => undefined,
    toRecord: () => state,
  },
  fires,
  requestShutdown: () => {},
  getSecret: () => "",
  strict: false,
  capabilities: {},
  workflowCapabilities: {},
  resources: {},
  missingSecrets: [],
});

describe("ac-template-engine", () => {
  it("passes through text without placeholders", async () => {
    const text = await Effect.runPromise(resolveTemplate("hello world", [], makeCtx()));
    expect(text).toBe("hello world");
  });

  it("preserves Jinja-style known prefix placeholders", async () => {
    const ctx = makeCtx({ region: "us-west" }, { token: "abc" });
    const text = await Effect.runPromise(
      resolveTemplate("${config.region}-${secret.token}", [], ctx),
    );
    expect(text).toBe("${config.region}-${secret.token}");
  });

  it("preserves event placeholders", async () => {
    const ctx = makeCtx();
    const event = new Event("build.done", { ref: "main", ok: true }, "ci");
    const text = await Effect.runPromise(
      resolveTemplate("${event.type} from ${event.source}: ${event.payload.ref}", [event], ctx),
    );
    expect(text).toBe("${event.type} from ${event.source}: ${event.payload.ref}");
  });

  it("preserves node id and fires placeholders", async () => {
    const ctx = makeCtx({}, {}, {}, 2);
    const node = new Node("n1", "shell", "", ["start"], ["ok"]);
    const text = await Effect.runPromise(
      resolveTemplate("${node.id} fired ${node.fires}", [Event.start()], ctx, node),
    );
    expect(text).toBe("${node.id} fired ${node.fires}");
  });

  it("preserves wildcard shard placeholders", async () => {
    const ctx = makeCtx();
    const node = new Node("n1", "shell", "", ["plugin.*.start"], ["ok"]);
    const event = new Event("plugin.42.start", {});
    const text = await Effect.runPromise(resolveValueForNode("${event.shard}", [event], ctx, node));
    expect(text).toBe("${event.shard}");
  });

  it("preserves bash variables and default syntax", async () => {
    const ctx = makeCtx();
    const home = await Effect.runPromise(resolveTemplate("echo ${HOME}", [], ctx));
    expect(home).toBe("echo ${HOME}");
    const worktree = await Effect.runPromise(resolveTemplate("echo ${WORKTREE:-/tmp}", [], ctx));
    expect(worktree).toBe("echo ${WORKTREE:-/tmp}");
    const path = await Effect.runPromise(resolveTemplate("echo $PATH", [], ctx));
    expect(path).toBe("echo $PATH");
  });

  it("preserves a mix of bash and known placeholders", async () => {
    const ctx = makeCtx({ attempts: 3 });
    const text = await Effect.runPromise(
      resolveTemplate("echo '${ATTEMPT}/${config.attempts}' in ${WORKTREE:-.}", [], ctx),
    );
    expect(text).toBe("echo '${ATTEMPT}/${config.attempts}' in ${WORKTREE:-.}");
  });

  it("preserves unknown placeholders", async () => {
    const ctx = makeCtx();
    const text = await Effect.runPromise(resolveTemplate("${notknown}", [], ctx));
    expect(text).toBe("${notknown}");
  });

  it("preserves Jinja-style pipe filter placeholders", async () => {
    const ctx = makeCtx({ name: "  ada  " });
    const text = await Effect.runPromise(resolveTemplate("${config.name | trim | upper}", [], ctx));
    expect(text).toBe("${config.name | trim | upper}");
  });

  it("preserves file and shell filter placeholders", async () => {
    const ctx = makeCtx();
    const file = await Effect.runPromise(resolveTemplate('${ "/tmp/data.txt" | file }', [], ctx));
    expect(file).toBe('${ "/tmp/data.txt" | file }');
    const shell = await Effect.runPromise(resolveTemplate('${ "echo hi" | shell }', [], ctx));
    expect(shell).toBe('${ "echo hi" | shell }');
  });

  it("recurses into dicts and lists via resolveValue", async () => {
    const ctx = makeCtx({ x: "X" }, { y: "Y" });
    const value = await Effect.runPromise(
      resolveValue(
        {
          a: "${config.x}",
          b: ["${secret.y}", 123],
          c: { d: "literal" },
        },
        [],
        ctx,
      ),
    );
    expect(value).toEqual({
      a: "${config.x}",
      b: ["${secret.y}", 123],
      c: { d: "literal" },
    });
  });
});
