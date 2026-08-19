import cp from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { Effect, Layer } from "effect";
import { Driver, DriverError, Event, type ExecutionContext, type Node } from "ac-core";
import { resolveValueForNode } from "ac-template-engine";

export const AgentDriver: Driver = {
  name: "agent",
  invoke: (node: Node, triggers: ReadonlyArray<Event>, ctx: ExecutionContext) =>
    Effect.gen(function* () {
      const resolved = yield* resolveValueForNode(node.with_, triggers, ctx, node).pipe(
        Effect.mapError((error) => new DriverError({ reason: String(error) })),
      );

      const with_ = resolved as Record<string, unknown>;
      const runner = typeof with_.runner === "string" ? with_.runner : "devin";
      const cwd = typeof with_.cwd === "string" ? with_.cwd : process.cwd();
      const timeoutMs = (typeof with_.timeout === "number" ? with_.timeout : 600) * 1000;

      const resolvedEnv = with_.env as Record<string, string> | undefined;
      const requestedSecrets = Array.isArray(with_.secrets)
        ? (with_.secrets as ReadonlyArray<unknown>)
        : [];
      const env: Record<string, string | undefined> = { ...process.env, ...resolvedEnv };
      for (const key of requestedSecrets) {
        if (typeof key === "string" && key in ctx.secrets) {
          env[key] = ctx.secrets[key];
        }
      }

      let promptText = "";

      if (typeof with_.prompt === "string" && with_.prompt.length > 0) {
        promptText = with_.prompt;
      } else if (typeof with_.prompt_file === "string" && with_.prompt_file.length > 0) {
        const filePath = path.isAbsolute(with_.prompt_file)
          ? with_.prompt_file
          : path.join(process.cwd(), with_.prompt_file);
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
            reason: `agent node '${node.id}' missing 'with.prompt' or 'with.prompt_file'`,
          }),
        );
      }

      const permissionMode =
        typeof with_.permission_mode === "string" ? with_.permission_mode : undefined;
      const trust = with_.trust === true;

      let binary = "";
      let argv: string[] = [];
      let input: string | undefined;

      if (runner === "devin") {
        binary = "devin";
        argv = ["-p"];

        if (typeof with_.prompt_file === "string" && with_.prompt_file.length > 0) {
          argv.push("--prompt-file", with_.prompt_file);
        } else {
          argv.push(promptText);
        }

        if (permissionMode !== undefined) {
          argv.push("--permission-mode", permissionMode);
        }

        argv.push("--respect-workspace-trust", trust ? "true" : "false");
      } else if (runner === "custom") {
        if (typeof with_.binary !== "string" || with_.binary.length === 0) {
          return yield* Effect.fail(
            new DriverError({
              reason: `agent node '${node.id}' custom runner missing 'with.binary'`,
            }),
          );
        }

        binary = with_.binary;

        if (
          !Array.isArray(with_.flags) ||
          !(with_.flags as ReadonlyArray<unknown>).every((item) => typeof item === "string")
        ) {
          return yield* Effect.fail(
            new DriverError({
              reason: `agent node '${node.id}' custom runner 'with.flags' must be an array of strings`,
            }),
          );
        }

        argv = [...(with_.flags as ReadonlyArray<string>)];

        if (with_.extra_args !== undefined) {
          if (
            !Array.isArray(with_.extra_args) ||
            !(with_.extra_args as ReadonlyArray<unknown>).every((item) => typeof item === "string")
          ) {
            return yield* Effect.fail(
              new DriverError({
                reason: `agent node '${node.id}' custom runner 'with.extra_args' must be an array of strings`,
              }),
            );
          }

          argv.push(...(with_.extra_args as ReadonlyArray<string>));
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
        const emit = with_.emit as Record<string, string> | undefined;

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
    }),
};

export const AgentDriverLive = Layer.succeed(Driver, AgentDriver);
