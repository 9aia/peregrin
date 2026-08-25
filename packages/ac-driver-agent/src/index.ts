import cp from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { Effect, Layer } from "effect";
import { Driver, DriverError, Event, type ExecutionContext, type Node } from "handl-core";
import { resolveValueForNode } from "handl-template-engine";
import { LiteLLM, Live as LiteLLMLive } from "handl-plugin-litellm";

export const AgentDriver: Driver = {
  name: "agent",
  invoke: (node: Node, triggers: ReadonlyArray<Event>, ctx: ExecutionContext) =>
    Effect.gen(function* () {
      const resolved = yield* resolveValueForNode(node.params, triggers, ctx, node).pipe(
        Effect.mapError((error) => new DriverError({ reason: String(error) })),
      );

      const params = resolved as Record<string, unknown>;
      const runner = typeof params.runner === "string" ? params.runner : "devin";
      const cwd = typeof params.cwd === "string" ? params.cwd : process.cwd();
      const timeoutMs = (typeof params.timeout === "number" ? params.timeout : 600) * 1000;

      const resolvedEnv = params.env as Record<string, string> | undefined;
      const requestedSecrets = Array.isArray(params.secrets)
        ? (params.secrets as ReadonlyArray<unknown>)
        : [];
      const env: Record<string, string | undefined> = { ...process.env, ...resolvedEnv };
      for (const key of requestedSecrets) {
        if (typeof key === "string" && key in ctx.secrets) {
          env[key] = ctx.secrets[key];
        }
      }

      let promptText = "";

      if (typeof params.prompt === "string" && params.prompt.length > 0) {
        promptText = params.prompt;
      } else if (typeof params.prompt_file === "string" && params.prompt_file.length > 0) {
        const filePath = path.isAbsolute(params.prompt_file)
          ? params.prompt_file
          : path.join(process.cwd(), params.prompt_file);
        promptText = yield* Effect.try({
          try: () => fs.readFileSync(filePath, "utf-8"),
          catch: (error) =>
            new DriverError({
              reason: `agent node '${node.id}' failed to read prompt_file: ${String(error)}`,
            }),
        });
      } else {
        return yield* Effect.fail(
          new DriverError({
            reason: `agent node '${node.id}' missing 'params.prompt' or 'params.prompt_file'`,
          }),
        );
      }

      const model = params.model;
      const provider = params.provider;
      if (typeof model === "string" && model.length > 0) {
        if (ctx.dryRun) {
          return [
            new Event("agent.litellm", { content: "", model, provider, dry_run: true }, node.id),
          ];
        }

        const liteLLM = yield* LiteLLM;
        const content = yield* liteLLM.complete({
          model,
          provider: typeof provider === "string" && provider.length > 0 ? provider : undefined,
          prompt: promptText,
          endpoint:
            typeof params.endpoint === "string" && params.endpoint.length > 0
              ? params.endpoint
              : undefined,
          apiKey:
            typeof params.api_key === "string" && params.api_key.length > 0
              ? params.api_key
              : env.LITELLM_API_KEY,
          system:
            typeof params.system === "string" && params.system.length > 0
              ? params.system
              : undefined,
        });

        return [new Event("agent.litellm", { content, model, provider }, node.id)];
      }

      const permissionMode =
        typeof params.permission_mode === "string" ? params.permission_mode : undefined;
      const trust = params.trust === true;

      let binary = "";
      let argv: string[] = [];
      let input: string | undefined;

      if (runner === "devin") {
        binary = "devin";
        argv = ["-p"];

        if (typeof params.prompt_file === "string" && params.prompt_file.length > 0) {
          argv.push("--prompt-file", params.prompt_file);
        } else {
          argv.push(promptText);
        }

        if (permissionMode !== undefined) {
          argv.push("--permission-mode", permissionMode);
        }

        argv.push("--respect-workspace-trust", trust ? "true" : "false");
      } else if (runner === "custom") {
        if (typeof params.binary !== "string" || params.binary.length === 0) {
          return yield* Effect.fail(
            new DriverError({
              reason: `agent node '${node.id}' custom runner missing 'params.binary'`,
            }),
          );
        }

        binary = params.binary;

        if (
          !Array.isArray(params.flags) ||
          !(params.flags as ReadonlyArray<unknown>).every((item) => typeof item === "string")
        ) {
          return yield* Effect.fail(
            new DriverError({
              reason: `agent node '${node.id}' custom runner 'params.flags' must be an array of strings`,
            }),
          );
        }

        argv = [...(params.flags as ReadonlyArray<string>)];

        if (params.extra_args !== undefined) {
          if (
            !Array.isArray(params.extra_args) ||
            !(params.extra_args as ReadonlyArray<unknown>).every((item) => typeof item === "string")
          ) {
            return yield* Effect.fail(
              new DriverError({
                reason: `agent node '${node.id}' custom runner 'params.extra_args' must be an array of strings`,
              }),
            );
          }

          argv.push(...(params.extra_args as ReadonlyArray<string>));
        }

        if (promptText.length > 0) {
          input = promptText;
        }
      } else {
        return yield* Effect.fail(
          new DriverError({
            reason: `unknown agent runner '${runner}'`,
          }),
        );
      }

      const getEventType = (rc: number): string => {
        const emit = params.emit as Record<string, string> | undefined;

        if (emit && typeof emit === "object" && !Array.isArray(emit)) {
          if (emit[String(rc)] !== undefined) return emit[String(rc)];
          if (emit.default !== undefined) return emit.default;
        }

        if (rc === 0) {
          return node.emits[0] ?? `${node.id}.done`;
        }

        return node.emits[1] ?? node.emits[0] ?? `${node.id}.done`;
      };

      if (ctx.dryRun) {
        const eventType = getEventType(0);
        return [
          new Event(
            eventType,
            {
              cmd: argv.join(" "),
              dry_run: true,
              rc: 0,
              stdout: "",
              stderr: "",
              runner,
            },
            node.id,
          ),
        ];
      }

      const result = yield* Effect.try({
        try: () =>
          cp.spawnSync(binary, argv, {
            cwd,
            env,
            input,
            timeout: timeoutMs,
            encoding: "utf-8",
          }),
        catch: (error) =>
          new DriverError({
            reason: `agent node '${node.id}' failed to spawn: ${String(error)}`,
          }),
      });

      if (result.error) {
        return [new Event("node.failed", { error: String(result.error), rc: -1, runner }, node.id)];
      }

      const rc = result.status ?? -1;
      const stdout = result.stdout ?? "";
      const stderr = result.stderr ?? "";
      const eventType = getEventType(rc);

      return [new Event(eventType, { stdout, stderr, rc, runner }, node.id)];
    }).pipe(Effect.provide(LiteLLMLive)),
};

export const AgentDriverLive = Layer.succeed(Driver, AgentDriver);
