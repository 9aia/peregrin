import { expect, it } from "vite-plus/test";
import fs from "node:fs";
import path from "node:path";

it("exposes a cli bin entry", () => {
  const pkg = JSON.parse(
    fs.readFileSync(path.join(import.meta.dirname, "../package.json"), "utf8"),
  );
  expect(pkg.bin.ac).toContain("cli.mjs");
});
