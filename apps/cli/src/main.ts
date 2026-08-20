import { Args, Command, Options } from "@effect/cli";
import { NodeContext, NodeRuntime } from "@effect/platform-node";
import { Console, Effect, Layer, Option } from "effect";
import { Event, Observer } from "ac-core";
import { loadWorkflow, LoaderError, loadRaw } from "ac-loader-yaml";
import { discover } from "ac-file-router";
import { Runner, type RunOptions } from "ac-engine";
import {
  makeDefaultsLive,
  MetricsObserver,
  NoopObserverLayer,
  OtelObserver,
} from "ac-cli-defaults";
import fs from "node:fs";
import path from "node:path";

const workflowArg = Args.text({ name: "workflow" }).pipe(
  Args.withDescription("Path to the workflow YAML"),
);

const replayOption = Options.optional(
  Options.text("replay").pipe(Options.withDescription("Path to a JSON/JSONL event log to replay")),
);

const dryRunOption = Options.boolean("dry-run", { ifPresent: true }).pipe(
  Options.withDescription("Simulate workflow without side effects"),
);

const otelOption = Options.boolean("otel", { ifPresent: true }).pipe(
  Options.withDescription("Enable OpenTelemetry observer"),
);

const otelEndpointOption = Options.optional(
  Options.text("otel-endpoint").pipe(
    Options.withDescription("OTLP collector endpoint (e.g. http://collector:4318/v1/traces)"),
  ),
);

const otelProtocolOption = Options.optional(
  Options.text("otel-protocol").pipe(Options.withDescription("OTLP protocol: grpc or http")),
);

const otelServiceNameOption = Options.optional(
  Options.text("otel-service-name").pipe(Options.withDescription("OTLP service name")),
);

const logFormatOption = Options.optional(
  Options.text("log-format").pipe(Options.withDescription("Log format: text or json")),
);

const metricsOption = Options.boolean("metrics", { ifPresent: true }).pipe(
  Options.withDescription("Expose Prometheus /metrics and /health endpoints"),
);

const metricsPortOption = Options.optional(
  Options.text("metrics-port").pipe(
    Options.withDescription("Port for the metrics HTTP server (default: 9090)"),
  ),
);

const loadReplayEvents = (
  filePath: string,
): Effect.Effect<ReadonlyArray<Event>, LoaderError, never> =>
  Effect.gen(function* () {
    const content = yield* Effect.try(() => fs.readFileSync(filePath, "utf8")).pipe(
      Effect.mapError(
        (error) =>
          new LoaderError({ reason: `Failed to read replay log: ${filePath}`, cause: error }),
      ),
    );
    const raw = yield* Effect.try(() => {
      const trimmed = content.trim();
      if (trimmed.startsWith("[")) {
        return JSON.parse(trimmed) as Array<Record<string, unknown>>;
      }
      return trimmed
        .split("\n")
        .filter((line) => line.trim() !== "")
        .map((line) => JSON.parse(line) as Record<string, unknown>);
    }).pipe(
      Effect.mapError(
        (error) => new LoaderError({ reason: "Failed to parse replay log", cause: error }),
      ),
    );
    return raw.map(
      (record) =>
        new Event(
          typeof record.type === "string" ? record.type : "",
          (record.payload ?? {}) as Record<string, unknown>,
          typeof record.source === "string" ? record.source : "",
          typeof record.id === "string" ? record.id : "",
          typeof record.ts === "number" ? record.ts : 0,
        ),
    );
  });

const parseMetricsPort = (value: Option.Option<string>): number => {
  if (Option.isNone(value)) return 9090;
  const parsed = Number(value.value);
  return Number.isNaN(parsed) || parsed <= 0 ? 9090 : parsed;
};

const selectObserver = (config: {
  otel: boolean;
  otelEndpoint: Option.Option<string>;
  otelProtocol: Option.Option<string>;
  otelServiceName: Option.Option<string>;
  logFormat: Option.Option<string>;
  metrics: boolean;
  metricsPort: Option.Option<string>;
}) => {
  const logFormat = Option.getOrElse(config.logFormat, () => "text");
  if (config.metrics) {
    const port = parseMetricsPort(config.metricsPort);
    return Layer.succeed(Observer, new MetricsObserver({ port, logFormat }));
  }
  if (config.otel || Option.isSome(config.otelEndpoint)) {
    const endpoint = Option.match(config.otelEndpoint, {
      onNone: () => undefined,
      onSome: (value) => value,
    });
    const protocol = Option.match(config.otelProtocol, {
      onNone: () => undefined,
      onSome: (value) => value,
    });
    const serviceName = Option.match(config.otelServiceName, {
      onNone: () => undefined,
      onSome: (value) => value,
    });
    return Layer.succeed(
      Observer,
      new OtelObserver({
        endpoint,
        protocol,
        serviceName,
        logFormat,
      }),
    );
  }
  return NoopObserverLayer;
};

const run = Command.make(
  "run",
  {
    workflow: workflowArg,
    replay: replayOption,
    dryRun: dryRunOption,
    otel: otelOption,
    otelEndpoint: otelEndpointOption,
    otelProtocol: otelProtocolOption,
    otelServiceName: otelServiceNameOption,
    logFormat: logFormatOption,
    metrics: metricsOption,
    metricsPort: metricsPortOption,
  },
  ({
    workflow,
    replay,
    dryRun,
    otel,
    otelEndpoint,
    otelProtocol,
    otelServiceName,
    logFormat,
    metrics,
    metricsPort,
  }) =>
    Effect.gen(function* () {
      const wf = yield* loadWorkflow(workflow);
      let replayEvents: ReadonlyArray<Event> = [];
      if (Option.isSome(replay)) {
        replayEvents = yield* loadReplayEvents(replay.value);
      }
      const runOptions: RunOptions = {
        ...(replayEvents.length > 0 ? { replayEvents } : {}),
        dryRun,
      };
      const observer = selectObserver({
        otel,
        otelEndpoint,
        otelProtocol,
        otelServiceName,
        logFormat,
        metrics,
        metricsPort,
      });
      const runWorkflow = Effect.gen(function* () {
        const runner = yield* Runner;
        const events = yield* runner.run(wf, runOptions);
        for (const event of events) {
          yield* Console.log(`${event.ts} ${event.source} ${event.type}`);
        }
      });
      yield* runWorkflow.pipe(Effect.provide(makeDefaultsLive(observer)));
    }),
).pipe(Command.withDescription("Run a workflow"));

const validate = Command.make("validate", { workflow: workflowArg }, ({ workflow }) =>
  Effect.gen(function* () {
    const wf = yield* loadWorkflow(workflow);
    yield* Console.log(`workflow: ${wf.name || "(unnamed)"}`);
    yield* Console.log(`  nodes:  ${wf.nodes.length}`);
    yield* Console.log(`  events: ${wf.events.length}`);
    if (wf.imports.length > 0) {
      yield* Console.log(`  includes: ${wf.imports.join(", ")}`);
    }
    if (wf.stopOn.length > 0) {
      yield* Console.log(`  stop_on: ${wf.stopOn.join(", ")}`);
    }
    for (const node of wf.nodes) {
      const maxFiresStr = node.maxFires !== 1 ? ` max_fires=${node.maxFires}` : "";
      const untilStr = node.until.length > 0 ? ` until=${node.until.join(",")}` : "";
      const retireStr = node.retireOn.length > 0 ? ` retire_on=${node.retireOn.join(",")}` : "";
      const pollStr = node.poll ? ` poll=${JSON.stringify(node.poll)}` : "";
      yield* Console.log(
        `    [${node.id}] driver=${node.driver} handler=${node.handler} ` +
          `on=${JSON.stringify(node.on)} emits=${JSON.stringify(node.emits)} mode=${node.mode}` +
          `${maxFiresStr}${untilStr}${retireStr}${pollStr}`,
      );
    }
    const emitTypes = new Set<string>();
    for (const n of wf.nodes) {
      for (const e of n.emits) {
        emitTypes.add(e);
      }
    }
    for (const e of wf.events) {
      emitTypes.add(e.type);
    }
    for (const e of [
      "start",
      "node.failed",
      "node.exhausted",
      "workflow.done",
      "workflow.failed",
      "workflow.cancel",
      "workflow.cancelled",
      "workflow.cycle_detected",
      "event.invalid",
      "dlq.event",
    ]) {
      emitTypes.add(e);
    }
    for (const n of wf.nodes) {
      for (const pattern of n.on) {
        if (pattern !== "*" && !emitTypes.has(pattern)) {
          yield* Console.log(
            `  WARNING: node '${n.id}' subscribes to '${pattern}' but no node emits it`,
          );
        }
      }
    }
    yield* Console.log("OK");
  }),
).pipe(Command.withDescription("validate a workflow YAML"));

const stdoutOption = Options.boolean("stdout", { ifPresent: true }).pipe(
  Options.withDescription("Print root schema to stdout instead of writing files"),
);

const noHandlersOption = Options.boolean("no-handlers", { ifPresent: true }).pipe(
  Options.withDescription("Skip handler discovery"),
);

const DRIVER_TYPES = [
  "agent",
  "cli",
  "http",
  "human",
  "noop",
  "polling",
  "python",
  "runner",
  "shell",
  "stdin",
  "streaming",
  "webhook",
];

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const buildDefs = (
  eventTypes: ReadonlyArray<string>,
  snippetNames: ReadonlyArray<string>,
  handlerNames: ReadonlyArray<string>,
  payloadSchemas: Record<string, string>,
): Record<string, unknown> => {
  const handlerProp =
    handlerNames.length > 0 ? { type: "string", enum: handlerNames } : { type: "string" };
  const snippetProp =
    snippetNames.length > 0 ? { type: "string", enum: snippetNames } : { type: "string" };
  const placeholder = { type: "string", pattern: ".*\\$\\{.*\\}.*" };
  const anyOf = (...schemas: Array<Record<string, unknown>>) => ({
    anyOf: [...schemas, placeholder],
  });

  const nodeProps: Record<string, unknown> = {
    id: { type: "string" },
    driver: anyOf({ type: "string", enum: DRIVER_TYPES }),
    handler: anyOf(handlerProp),
    on: anyOf({
      type: "array",
      items: {
        anyOf: [
          { type: "string", enum: eventTypes },
          { type: "string", pattern: "^\\*\\*?$" },
          placeholder,
        ],
      },
    }),
    emits: anyOf({ type: "array", items: { type: "string" } }),
    snippet: anyOf(snippetProp),
    mode: anyOf({ type: "string", enum: ["any", "all"] }),
    with: { type: "object" },
    max_fires: anyOf({ type: "integer", minimum: 1 }, { type: "string", enum: ["unlimited"] }),
    until: anyOf({
      type: "array",
      items: {
        anyOf: [
          { type: "string", enum: eventTypes },
          { type: "string", pattern: "^\\*\\*?$" },
          placeholder,
        ],
      },
    }),
    retire_on: anyOf({
      type: "array",
      items: {
        anyOf: [
          { type: "string", enum: eventTypes },
          { type: "string", pattern: "^\\*\\*?$" },
          placeholder,
        ],
      },
    }),
    retry: {
      type: "object",
      properties: {
        max_attempts: { type: "integer" },
        backoff: { type: "string" },
        delay: { type: "number" },
      },
    },
    circuit_breaker: {
      type: "object",
      properties: {
        threshold: { type: "integer" },
        reset_after: { type: "number" },
        on_trip: { type: "string" },
      },
    },
    poll: {
      type: "object",
      properties: {
        interval: { type: "number" },
        until: { type: "array", items: { type: "string" } },
      },
    },
  };

  const snippetProperties: Record<string, unknown> = {
    driver: anyOf({ type: "string", enum: DRIVER_TYPES }),
    handler: anyOf(handlerProp),
    on: anyOf({
      type: "array",
      items: {
        anyOf: [
          { type: "string", enum: eventTypes },
          { type: "string", pattern: "^\\*\\*?$" },
          placeholder,
        ],
      },
    }),
    emits: anyOf({ type: "array", items: { type: "string" } }),
    snippet: anyOf(snippetProp),
    with: { type: "object" },
  };

  const snippetPropertiesMap: Record<string, unknown> = {};
  for (const name of snippetNames) {
    snippetPropertiesMap[name] = {
      type: ["object", "string", "array", "integer", "boolean"],
    };
  }

  const defs: Record<string, unknown> = {
    $schema: "http://json-schema.org/draft-07/schema#",
    $id: "_defs.schema.json",
    title: "agent-campaign shared definitions",
    $defs: {
      node: {
        type: ["object", "array"],
        properties: nodeProps,
        items: {
          type: "object",
          properties: nodeProps,
          required: ["id"],
        },
      },
      snippet: {
        type: ["object", "string", "array", "integer", "boolean"],
        properties: snippetProperties,
      },
      eventEnum: anyOf({ type: "string", enum: eventTypes }),
      driverEnum: { type: "string", enum: DRIVER_TYPES },
      handlerEnum: handlerProp,
      snippetEnum: snippetProp,
      placeholder,
    },
  };

  for (const [eventType, schemaPath] of Object.entries(payloadSchemas)) {
    (defs.$defs as Record<string, unknown>)[`payload:${eventType}`] = { $ref: schemaPath };
  }

  return {
    ...defs,
    snippets: {
      type: "object",
      properties: snippetPropertiesMap,
    },
  };
};

const typegen = Command.make(
  "typegen",
  {
    workflow: workflowArg,
    stdout: stdoutOption,
    noHandlers: noHandlersOption,
  },
  ({ workflow, stdout, noHandlers }) =>
    Effect.gen(function* () {
      const text = yield* Effect.try(() => fs.readFileSync(workflow, "utf8")).pipe(
        Effect.mapError(
          (error) =>
            new LoaderError({ reason: `Failed to read workflow: ${workflow}`, cause: error }),
        ),
      );
      const baseDir = path.dirname(path.resolve(workflow));
      const wfFilename = path.basename(workflow);
      const raw = yield* loadRaw(text, baseDir);
      const discovered = yield* discover(baseDir).pipe(
        Effect.mapError((error) => new LoaderError({ reason: error.reason })),
      );

      const snippetNames = new Set<string>();
      for (const entry of discovered.snippets) {
        snippetNames.add(entry.name);
      }
      if (isRecord(raw.snippets)) {
        for (const name of Object.keys(raw.snippets)) {
          snippetNames.add(name);
        }
      }

      const eventTypes = new Set<string>([
        "start",
        "workflow.done",
        "workflow.cancel",
        "node.failed",
        "workflow.failed",
        "event.invalid",
      ]);

      const rawEvents = Array.isArray(raw.events) ? raw.events : [];
      for (const e of rawEvents) {
        if (isRecord(e) && typeof e.type === "string" && !eventTypes.has(e.type)) {
          eventTypes.add(e.type);
        }
      }

      const rawNodes = Array.isArray(raw.nodes) ? raw.nodes : [];
      for (const n of rawNodes) {
        if (!isRecord(n)) continue;
        const emits = Array.isArray(n.emits) ? n.emits : [];
        for (const emit of emits) {
          if (typeof emit === "string" && !eventTypes.has(emit)) {
            eventTypes.add(emit);
          }
        }
      }

      const handlerNames = noHandlers ? [] : discovered.handlers.map((entry) => entry.name).sort();

      const defsDir = path.join(baseDir, ".ac", "generated", "schemas");
      const payloadSchemaRefs: Record<string, string> = {};
      for (const entry of discovered.events) {
        const rel = path.relative(defsDir, path.dirname(entry.path)).replace(/\\/g, "/");
        payloadSchemaRefs[entry.name] = path.posix.join(rel, path.basename(entry.path));
      }

      const eventTypesList = [...eventTypes].sort();
      const snippetNamesList = [...snippetNames].sort();
      const defs = buildDefs(eventTypesList, snippetNamesList, handlerNames, payloadSchemaRefs);

      const rootSchema: Record<string, unknown> = {
        $schema: "http://json-schema.org/draft-07/schema#",
        $id: `file://${workflow}`,
        title: `agent-campaign workflow: ${typeof raw.name === "string" ? raw.name : wfFilename}`,
        type: "object",
        properties: {
          name: { type: "string" },
          include: {
            type: ["string", "array"],
            items: { type: "string" },
            description: "YAML files to merge (dict-level)",
          },
          import: {
            type: ["string", "array"],
            items: {
              type: ["string", "object"],
              properties: { file: { type: "string" }, as: { type: "string" } },
            },
            description: "Python modules to import for handler registration",
          },
          stop_on: {
            type: "array",
            items: { $ref: "_defs.schema.json#/$defs/eventEnum" },
          },
          events: {
            type: "array",
            items: {
              type: "object",
              properties: {
                type: { $ref: "_defs.schema.json#/$defs/eventEnum" },
                fields: {
                  type: "object",
                  additionalProperties: {
                    type: "string",
                    pattern: "^(string|integer|number|boolean|array|object|null)\\??$",
                  },
                },
                schema: { type: "object" },
              },
              required: ["type"],
            },
          },
          snippets: {
            type: "object",
            description: "Named fragments",
            properties: defs.snippets ? (defs.snippets as Record<string, unknown>).properties : {},
          },
          config: { type: "object" },
          strict: { type: "boolean" },
          state: { type: "object" },
          payload_schemas: { type: "object" },
          nodes: {
            type: "array",
            items: { $ref: "_defs.schema.json#/$defs/node" },
          },
        },
        additionalProperties: false,
      };

      if (stdout) {
        yield* Console.log(JSON.stringify(rootSchema, null, 2));
        return;
      }

      yield* Effect.try(() => fs.mkdirSync(defsDir, { recursive: true })).pipe(
        Effect.mapError(
          (error) => new LoaderError({ reason: `Failed to create ${defsDir}`, cause: error }),
        ),
      );

      const written: Array<string> = [];

      const writeSchema = (
        schema: Record<string, unknown>,
        relPath: string,
      ): Effect.Effect<void, LoaderError, never> =>
        Effect.try(() => {
          const full = path.join(defsDir, relPath);
          fs.mkdirSync(path.dirname(full), { recursive: true });
          fs.writeFileSync(full, `${JSON.stringify(schema, null, 2)}\n`, "utf8");
          written.push(relPath);
        }).pipe(
          Effect.mapError(
            (error) =>
              new LoaderError({ reason: `Failed to write schema ${relPath}`, cause: error }),
          ),
        );

      const injectSchemaComment = (
        yamlPath: string,
        schemaRelPath: string,
      ): Effect.Effect<void, LoaderError, never> => {
        const schemaLine = `# yaml-language-server: $schema=${schemaRelPath}`;
        return Effect.try(() => {
          const content = fs.readFileSync(yamlPath, "utf8");
          const hasSchema = content
            .split("\n")
            .some((line) => line.trim().startsWith("# yaml-language-server: $schema="));
          if (!hasSchema) {
            fs.writeFileSync(yamlPath, `${schemaLine}\n${content}`, "utf8");
          }
        }).pipe(
          Effect.mapError(
            (error) => new LoaderError({ reason: `Failed to annotate ${yamlPath}`, cause: error }),
          ),
        );
      };

      const { snippets, ...defsWithoutSnippets } = defs;
      void snippets;

      yield* writeSchema(defsWithoutSnippets, "_defs.schema.json");

      const rootSchemaRel = `${path.parse(wfFilename).name}.schema.json`;
      yield* writeSchema(rootSchema, rootSchemaRel);
      yield* injectSchemaComment(
        workflow,
        path.posix.join(".ac", "generated", "schemas", rootSchemaRel),
      );

      for (const entry of discovered.nodes) {
        const rel = `nodes/${entry.name}.schema.json`;
        yield* writeSchema({ $ref: "_defs.schema.json#/$defs/node" }, rel);
        yield* injectSchemaComment(entry.path, path.posix.join(".ac", "generated", "schemas", rel));
      }

      for (const entry of discovered.snippets) {
        const rel = `snippets/${entry.name}.schema.json`;
        yield* writeSchema({ $ref: "_defs.schema.json#/$defs/snippet" }, rel);
        yield* injectSchemaComment(entry.path, path.posix.join(".ac", "generated", "schemas", rel));
      }

      yield* Console.log(`schemas written to ${defsDir}`);
      yield* Console.log(`  events:   ${eventTypesList.length} types`);
      yield* Console.log(`  snippets: ${snippetNamesList.length} names`);
      yield* Console.log(`  handlers: ${handlerNames.length} registered`);
      yield* Console.log(`  drivers:  ${DRIVER_TYPES.length} types`);
      yield* Console.log(`  files:    ${written.length} schemas generated`);
      for (const rel of written) {
        yield* Console.log(`    .ac/generated/schemas/${rel}`);
      }
    }),
).pipe(Command.withDescription("generate JSON Schemas for a workflow"));

const version = Command.make("version", {}, () => Console.log("0.0.0")).pipe(
  Command.withDescription("Show the CLI version"),
);

const ac = Command.make("ac").pipe(Command.withSubcommands([run, validate, typegen, version]));

const program = Command.run(ac, {
  name: "ac",
  version: "0.0.0",
});

NodeRuntime.runMain(Effect.provide(program(process.argv), NodeContext.layer));
