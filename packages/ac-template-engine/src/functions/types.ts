import type { Effect } from "effect";
import type { ExecutionContext, Node, TemplateError } from "ac-core";

export interface TemplateFunction {
  readonly name: string;
  readonly deterministic: boolean;
  readonly requiresCapability: boolean;
  readonly call: (
    args: ReadonlyArray<unknown>,
    ctx: ExecutionContext,
    node: Node | undefined,
    method: string | undefined,
  ) => Effect.Effect<unknown, TemplateError>;
}
