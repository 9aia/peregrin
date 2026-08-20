import { envFunction } from "./env.js";
import { fileFunction } from "./file.js";
import { nowFunction } from "./now.js";
import { randomFunction } from "./random.js";
import { shellFunction } from "./shell.js";
import { uuidFunction } from "./uuid.js";
import type { TemplateFunction } from "./types.js";

export type { TemplateFunction };

export const templateFunctions: Readonly<Record<string, TemplateFunction>> = {
  env: envFunction,
  file: fileFunction,
  now: nowFunction,
  random: randomFunction,
  shell: shellFunction,
  uuid: uuidFunction,
};
