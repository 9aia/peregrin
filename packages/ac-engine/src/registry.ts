import { Context, Effect, Layer } from "effect";
import { DriverError } from "ac-core";
import type { Driver } from "ac-core";

export interface DriverRegistry {
  readonly register: (name: string, driver: Driver) => void;
  readonly resolve: (name: string) => Effect.Effect<Driver, DriverError, never>;
}

export const DriverRegistry = Context.GenericTag<DriverRegistry>("ac/DriverRegistry");

export class DriverRegistryLive implements DriverRegistry {
  private drivers = new Map<string, Driver>();

  register(name: string, driver: Driver) {
    this.drivers.set(name, driver);
  }

  resolve(name: string) {
    return Effect.sync(() => this.drivers.get(name) ?? this.drivers.get(name.split(":")[0])).pipe(
      Effect.flatMap((driver) =>
        driver
          ? Effect.succeed(driver)
          : Effect.fail(new DriverError({ reason: `no driver registered for '${name}'` })),
      ),
    );
  }
}

export const DriverRegistryLiveLayer = Layer.succeed(DriverRegistry, new DriverRegistryLive());
