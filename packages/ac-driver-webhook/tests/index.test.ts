import { describe, it, expect } from "vite-plus/test";
import { WebhookDriver, WebhookDriverLive } from "../src/index.ts";

describe("handl-driver-webhook", () => {
  it("exports a driver named webhook", () => {
    expect(WebhookDriver.name).toBe("webhook");
  });

  it("exports a Live layer", () => {
    expect(WebhookDriverLive).toBeDefined();
  });
});
