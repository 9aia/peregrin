import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const root = path.resolve(path.dirname(__filename), "..");
const casesPath = path.join(root, "cases", "number_format.json");
const fixturesDir = path.join(root, "fixtures");
const fixturesPath = path.join(fixturesDir, "number_format.json");

const cases = JSON.parse(fs.readFileSync(casesPath, "utf-8"));

function resolveInput(raw) {
  if (typeof raw === "number") return raw;
  switch (raw) {
    case "NaN":
      return NaN;
    case "Infinity":
      return Infinity;
    case "-Infinity":
      return -Infinity;
    default:
      throw new Error(`unknown input literal: ${raw}`);
  }
}

function inputLabel(value) {
  if (Number.isNaN(value)) return "NaN";
  if (value === Infinity) return "Infinity";
  if (value === -Infinity) return "-Infinity";
  return value;
}

const results = [];

for (const testCase of cases) {
  const fmt = new Intl.NumberFormat(testCase.locale, testCase.options);
  const outputs = [];
  for (const raw of testCase.inputs) {
    const value = resolveInput(raw);
    const formatted = fmt.format(value);
    const parts = fmt.formatToParts(value).map((p) => ({ type: p.type, value: p.value }));
    outputs.push({ input: inputLabel(value), formatted, parts });
  }
  results.push({
    id: testCase.id,
    locale: testCase.locale,
    options: testCase.options,
    outputs,
  });
}

const fixture = {
  version: 1,
  generatedBy: process.version,
  generatedAt: new Date().toISOString(),
  cases: results,
};

fs.mkdirSync(fixturesDir, { recursive: true });
fs.writeFileSync(fixturesPath, JSON.stringify(fixture, null, 2));
console.log(`wrote ${fixturesPath}`);
