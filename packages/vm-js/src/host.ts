import { Context } from "./context.js";

export type HostFunction = (ctx: Context, ...args: unknown[]) => unknown;

export class UnknownFunctionError extends Error {
  constructor(
    public readonly name: string,
    public readonly args: unknown[] = [],
  ) {
    super(`unknown function: ${name}`);
  }
}

export class Host {
  private readonly functions = new Map<string, HostFunction>();

  register(name: string, fn: HostFunction): this {
    this.functions.set(name, fn);
    return this;
  }

  has(name: string): boolean {
    return this.functions.has(name);
  }

  call(ctx: Context, name: string, args: unknown[]): unknown {
    const fn = this.functions.get(name);
    if (!fn) throw new UnknownFunctionError(name, args);
    return fn(ctx, ...args);
  }
}
