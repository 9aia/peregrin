import { describe, it, expect } from "vite-plus/test";
import { Context, Host, Vm } from "../src/index.ts";

describe("vm-js", () => {
  it("runs a host function", () => {
    const vm = new Vm(new Host().register("math.add", (_ctx, a, b) => Number(a) + Number(b)));
    const ctx = vm.context();
    const gen = (function* (c: Context, v: Vm) {
      return yield* v.callGen(c, "math.add", [1, 2]);
    })(ctx, vm);
    expect(vm.run(ctx, gen)).toBe(3);
  });

  it("applies a function value", () => {
    const vm = new Vm(new Host().register("math.add", (_ctx, a, b) => Number(a) + Number(b)));
    const ctx = vm.context();
    const fn = {
      __call: function* (c: Context, v: Vm, args: unknown[]) {
        return yield* v.callGen(c, "math.add", [args[0], args[0]]);
      },
    };
    const gen = (function* (c: Context, v: Vm) {
      return yield* v.applyGen(c, fn, [5]);
    })(ctx, vm);
    expect(vm.run(ctx, gen)).toBe(10);
  });

  it("handles an operation with resume", () => {
    const vm = new Vm();
    const ctx = vm.context();
    const body = (function* (c: Context, v: Vm) {
      return yield* v.callGen(c, "boom", [1]);
    })(ctx, vm);
    const handlers = [
      {
        op: "boom",
        params: ["x"],
        body: (
          _c: Context,
          _v: Vm,
          _args: Record<string, unknown>,
          resume: (value: unknown) => unknown,
        ) => resume(42),
      },
    ];
    expect(vm.handle(ctx, body, handlers)).toBe(42);
  });

  it("resumes and continues computation", () => {
    const vm = new Vm(new Host().register("math.add", (_ctx, a, b) => Number(a) + Number(b)));
    const ctx = vm.context();
    const body = (function* (c: Context, v: Vm) {
      const a = (yield* v.callGen(c, "boom", [1])) as number;
      return yield* v.callGen(c, "math.add", [a, 10]);
    })(ctx, vm);
    const handlers = [
      {
        op: "boom",
        params: ["x"],
        body: (
          _c: Context,
          _v: Vm,
          args: Record<string, unknown>,
          resume: (value: unknown) => unknown,
        ) => resume(Number(args.x) * 2),
      },
    ];
    expect(vm.handle(ctx, body, handlers)).toBe(12);
  });
});
