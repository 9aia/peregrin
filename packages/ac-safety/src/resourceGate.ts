import { Duration, Effect } from "effect";
import type { Adapter } from "effect/Effect";

export type Probe = () => readonly [number, number, number];

export const defaultProbe: Probe = () => [0, 0, 1024];

export class LoadBreaker {
  readonly cpuThreshold: number;
  readonly memThreshold: number;
  readonly sustained: number;
  readonly cooldown: number;
  private readonly _probe: Probe;
  private _isOpen: boolean;
  private _openedAt: number;
  private _overSince: number | null;
  private _tripped: boolean;

  constructor(config: Readonly<Record<string, unknown>>, probe: Probe) {
    this.cpuThreshold = Number(config["cpu_threshold"] ?? 0);
    this.memThreshold = Number(config["mem_threshold"] ?? 0);
    this.sustained = Number(config["sustained_seconds"] ?? 10);
    this.cooldown = Number(config["cooldown"] ?? 30);
    this._probe = probe;
    this._isOpen = false;
    this._openedAt = 0;
    this._overSince = null;
    this._tripped = false;
  }

  update(now: number): boolean {
    if (this._isOpen && now - this._openedAt >= this.cooldown) {
      this._isOpen = false;
      this._overSince = null;
    }
    if (this._overloaded()) {
      if (this._overSince === null) {
        this._overSince = now;
      } else if (!this._isOpen && now - this._overSince >= this.sustained) {
        this._isOpen = true;
        this._openedAt = now;
        this._tripped = true;
      }
    } else {
      this._overSince = null;
    }
    return this._isOpen;
  }

  takeTrip(): boolean {
    const flag = this._tripped;
    this._tripped = false;
    return flag;
  }

  reset(): void {
    this._isOpen = false;
    this._openedAt = 0;
    this._overSince = null;
    this._tripped = false;
  }

  private _overloaded(): boolean {
    const [cpu, mem] = this._probe();
    if (this.cpuThreshold && cpu >= this.cpuThreshold) return true;
    if (this.memThreshold && mem >= this.memThreshold) return true;
    return false;
  }
}

export class ResourceGate {
  readonly maxConcurrency: number;
  readonly maxCpu: number;
  readonly maxMem: number;
  readonly minFreeMem: number;
  private readonly _probe: Probe;
  private readonly _breaker: LoadBreaker | null;
  private readonly _poll: number;
  private _inUse: number;

  constructor(
    maxConcurrency = 50,
    maxCpuPercent = 0,
    maxMemPercent = 0,
    minFreeMemMb = 0,
    probe: Probe = defaultProbe,
    breaker: LoadBreaker | null = null,
    pollInterval = 0.05,
  ) {
    this.maxConcurrency = maxConcurrency;
    this.maxCpu = maxCpuPercent;
    this.maxMem = maxMemPercent;
    this.minFreeMem = minFreeMemMb;
    this._probe = probe;
    this._breaker = breaker;
    this._poll = pollInterval;
    this._inUse = 0;
  }

  private _cap(weight: number): number {
    return Math.max(1, Math.min(weight, this.maxConcurrency));
  }

  private _canAdmit(weight: number): boolean {
    if (this._breaker !== null) {
      if (this._breaker.update(Date.now() / 1000)) return false;
    }
    if (this._inUse + weight > this.maxConcurrency) return false;
    if (this.maxCpu || this.maxMem || this.minFreeMem) {
      const [cpu, mem, free] = this._probe();
      if (this.maxCpu && cpu >= this.maxCpu) return false;
      if (this.maxMem && mem >= this.maxMem) return false;
      if (this.minFreeMem && free < this.minFreeMem) return false;
    }
    return true;
  }

  acquire(weight: number): Effect.Effect<number> {
    return Effect.gen(this, function* (this: ResourceGate, _resume: Adapter) {
      const capped = this._cap(weight);
      while (true) {
        if (yield* Effect.sync(() => this._canAdmit(capped))) break;
        yield* Effect.sleep(Duration.millis(this._poll * 1000));
      }
      yield* Effect.sync(() => {
        this._inUse += capped;
      });
      return capped;
    });
  }

  release(weight: number): void {
    const capped = this._cap(weight);
    this._inUse -= capped;
    if (this._inUse < 0) this._inUse = 0;
  }

  isOverloaded(): boolean {
    if (!(this.maxCpu || this.maxMem || this.minFreeMem)) return false;
    const [cpu, mem, free] = this._probe();
    if (this.maxCpu && cpu >= this.maxCpu) return true;
    if (this.maxMem && mem >= this.maxMem) return true;
    if (this.minFreeMem && free < this.minFreeMem) return true;
    return false;
  }
}
