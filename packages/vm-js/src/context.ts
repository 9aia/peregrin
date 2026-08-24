export class Context {
  private values: Record<string, unknown> = {};

  constructor(private readonly parent?: Context) {}

  define(name: string, value: unknown): void {
    this.values[name] = value;
  }

  get(name: string): unknown {
    if (name in this.values) return this.values[name];
    return this.parent?.get(name);
  }

  has(name: string): boolean {
    if (name in this.values) return true;
    return this.parent?.has(name) ?? false;
  }

  child(): Context {
    return new Context(this);
  }
}
