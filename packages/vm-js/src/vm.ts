import { Context } from "./context.js";
import { Host, UnknownFunctionError } from "./host.js";
import { bindPattern, type Pattern } from "./pattern.js";

export type Operation = {
  __op: true;
  name: string;
  args: unknown[];
};

export type Handler = {
  op: string;
  params: string[];
  body: (
    ctx: Context,
    vm: Vm,
    args: Record<string, unknown>,
    resume: (value: unknown) => unknown,
  ) => unknown;
};

export class Vm {
  constructor(public readonly host: Host = new Host()) {}

  context(): Context {
    return new Context();
  }

  resolve(ctx: Context, name: string): unknown {
    const value = ctx.get(name);
    if (value !== undefined) return value;
    if (this.host.has(name)) return this.host.call(ctx, name, []);
    throw new UnknownFunctionError(name);
  }

  bindPattern(ctx: Context, pattern: Pattern, value: unknown): unknown {
    bindPattern(ctx, pattern, value);
    return value;
  }

  *callGen(ctx: Context, name: string, args: unknown[]): Generator<unknown, unknown, unknown> {
    if (this.host.has(name)) return this.host.call(ctx, name, args);
    const fn = ctx.get(name);
    if (fn !== undefined) return yield* this.applyGen(ctx, fn, args);
    const $r = yield { __op: true, name, args };
    return $r;
  }

  *applyGen(ctx: Context, fn: unknown, args: unknown[]): Generator<unknown, unknown, unknown> {
    if (typeof fn === "string") return yield* this.callGen(ctx, fn, args);
    if (typeof fn === "object" && fn !== null && "__call" in fn) {
      const child = ctx.child();
      const record = fn as {
        __call: (ctx: Context, vm: Vm, args: unknown[]) => Generator<unknown, unknown, unknown>;
      };
      return yield* record.__call(child, this, args);
    }
    if (typeof fn === "function") return (fn as (...args: unknown[]) => unknown)(...args);
    throw new Error("not a function");
  }

  index(target: unknown, index: unknown): unknown {
    if (Array.isArray(target)) return target[index as number] ?? null;
    if (typeof target === "object" && target !== null)
      return (target as Record<string, unknown>)[index as string] ?? null;
    if (typeof target === "string") return (target as string)[index as number] ?? null;
    throw new Error("not indexable");
  }

  field(target: unknown, field: string): unknown {
    if (typeof target === "object" && target !== null)
      return (target as Record<string, unknown>)[field] ?? null;
    if (typeof target === "string") return `${target}.${field}`;
    throw new Error("not fieldable");
  }

  isOp(value: unknown): value is Operation {
    return (
      typeof value === "object" &&
      value !== null &&
      "__op" in value &&
      (value as { __op: boolean }).__op === true
    );
  }

  isIterator(value: unknown): value is Generator<unknown, unknown, unknown> {
    return (
      typeof value === "object" &&
      value !== null &&
      "next" in value &&
      typeof (value as { next: unknown }).next === "function"
    );
  }

  handle<T>(ctx: Context, body: Generator<unknown, T, unknown>, handlers: Handler[]): T {
    return this.drive(ctx, body, handlers) as T;
  }

  run<T>(ctx: Context, body: Generator<unknown, T, unknown>): T {
    return this.drive(ctx, body, []) as T;
  }

  drive(
    ctx: Context,
    gen: Generator<unknown, unknown, unknown>,
    handlers: Handler[],
    startValue?: unknown,
  ): unknown {
    let step = gen.next(startValue);
    while (!step.done) {
      const value = step.value;
      if (this.isOp(value)) {
        const h = handlers.find((h) => h.op === value.name);
        if (!h) throw new UnknownFunctionError(value.name, value.args);
        const child = ctx.child();
        const opArgs: Record<string, unknown> = {};
        for (let i = 0; i < h.params.length; i++) {
          opArgs[h.params[i]] = value.args[i];
        }
        const resume = (v: unknown) => this.drive(ctx, gen, handlers, v);
        const handlerResult = h.body(child, this, opArgs, resume);
        if (this.isIterator(handlerResult)) {
          return this.drive(ctx, handlerResult, handlers);
        }
        return handlerResult;
      }
      step = gen.next();
    }
    return step.value;
  }
}
