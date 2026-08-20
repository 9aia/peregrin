const LIST_FIELDS = new Set(["allow", "deny", "ports", "hosts", "paths"]);
const DICT_LIST_KEYS = new Set(["shell", "http", "python", "webhook"]);
const TOP_LIST_KEYS = new Set(["secrets", "agents", "urls"]);

function deepCopy(val: unknown): unknown {
  if (Array.isArray(val)) return [...val];
  if (val !== null && typeof val === "object") {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(val)) out[k] = deepCopy(v);
    return out;
  }
  return val;
}

function asList(val: unknown): ReadonlyArray<string> {
  if (!Array.isArray(val)) return [];
  return val.map((v) => asString(v));
}

function asString(val: unknown, fallback = ""): string {
  return typeof val === "string" ? val : fallback;
}

function asNumber(val: unknown, fallback: number): number {
  if (typeof val === "number") return val;
  if (typeof val === "string" && val.trim() !== "") return Number(val);
  return fallback;
}

function globMatch(pattern: string, value: string): boolean {
  const specials = new Set(["\\", "^", "$", ".", "|", "?", "*", "+", "(", ")", "[", "]", "{", "}"]);
  let out = "^";
  for (const c of pattern) {
    if (c === "*") out += ".*";
    else if (c === "?") out += ".";
    else if (specials.has(c)) out += "\\" + c;
    else out += c;
  }
  out += "$";
  return new RegExp(out).test(value);
}

export class CapabilityChecker {
  private readonly _wfCaps: Readonly<Record<string, unknown>>;
  private readonly _defaultPolicy: string;

  constructor(workflowCaps: Readonly<Record<string, unknown>> | null, strict = false) {
    this._wfCaps = workflowCaps ?? {};
    this._defaultPolicy = asString(this._wfCaps["default_policy"], strict ? "deny" : "allow");
  }

  merge(nodeCaps: Readonly<Record<string, unknown>> | null): Readonly<Record<string, unknown>> {
    const node = nodeCaps ?? {};
    const mergeLists = node["merge"] === true;
    const result: Record<string, unknown> = {};

    for (const [key, val] of Object.entries(this._wfCaps)) {
      result[key] = deepCopy(val);
    }

    for (const [key, val] of Object.entries(node)) {
      if (key === "merge") continue;
      if (DICT_LIST_KEYS.has(key)) {
        const wf = (result[key] as Readonly<Record<string, unknown>> | undefined) ?? {};
        const nv = (typeof val === "object" && val !== null ? val : {}) as Readonly<
          Record<string, unknown>
        >;
        result[key] = this._mergeDictList(wf, nv, mergeLists);
      } else if (TOP_LIST_KEYS.has(key)) {
        result[key] = this._mergeList(asList(result[key]), asList(val), mergeLists);
      } else {
        result[key] = val;
      }
    }

    return result;
  }

  check(driver: string, resolvedParams: Readonly<Record<string, unknown>>): ReadonlyArray<string> {
    if (this._defaultPolicy === "allow") return [];

    const violations: string[] = [];

    if (driver === "shell") {
      violations.push(...this._checkGlobField(asString(resolvedParams["cmd"]), "shell"));
    } else if (driver === "http") {
      violations.push(...this._checkGlobField(asString(resolvedParams["url"]), "http"));
    } else if (driver === "agent") {
      violations.push(...this._checkAgent(asString(resolvedParams["runner"])));
    } else if (driver === "python") {
      violations.push(...this._checkPython(resolvedParams));
    } else if (driver === "webhook") {
      violations.push(...this._checkWebhook(resolvedParams));
    }

    return violations;
  }

  filterSecrets(secrets: Readonly<Record<string, string>>): Record<string, string> {
    if (this._defaultPolicy === "allow") return { ...secrets };
    const allowed = asList(this._wfCaps["secrets"]);
    if (allowed.length === 0) return {};
    return Object.fromEntries(Object.entries(secrets).filter(([k]) => allowed.includes(k)));
  }

  private _mergeList(
    wfList: ReadonlyArray<unknown>,
    nodeList: ReadonlyArray<unknown>,
    merge: boolean,
  ): ReadonlyArray<unknown> {
    if (!merge) return [...nodeList];
    const combined = [...wfList];
    for (const item of nodeList) {
      if (!combined.includes(item)) combined.push(item);
    }
    return combined;
  }

  private _mergeDictList(
    wfDict: Readonly<Record<string, unknown>>,
    nodeDict: Readonly<Record<string, unknown>>,
    merge: boolean,
  ): Readonly<Record<string, unknown>> {
    const result: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(wfDict)) {
      result[k] = Array.isArray(v) ? [...v] : v;
    }
    for (const [k, v] of Object.entries(nodeDict)) {
      if (LIST_FIELDS.has(k) && Array.isArray(v)) {
        result[k] = this._mergeList(asList(result[k]), v as ReadonlyArray<unknown>, merge);
      } else {
        result[k] = v;
      }
    }
    return result;
  }

  private _checkPython(resolvedParams: Readonly<Record<string, unknown>>): ReadonlyArray<string> {
    const handler = asString(resolvedParams["handler"]);
    if (!handler) return ["python node missing handler name"];
    return this._checkGlobField(handler, "python");
  }

  private _checkWebhook(resolvedParams: Readonly<Record<string, unknown>>): ReadonlyArray<string> {
    const caps = (this._wfCaps["webhook"] ?? {}) as Readonly<Record<string, unknown>>;
    if (Object.keys(caps).length === 0) {
      return ["webhook blocked (deny-all, no webhook capabilities)"];
    }

    const violations: string[] = [];
    const port = asNumber(resolvedParams["port"], 8080);
    const host = asString(resolvedParams["host"], "127.0.0.1");
    const path = asString(resolvedParams["path"], "/events");
    const auth = resolvedParams["auth"] as Readonly<Record<string, unknown>> | undefined;

    const allowedPorts = asList(caps["ports"]);
    if (allowedPorts.length === 0) {
      violations.push(`port ${port} blocked (deny-all, no ports list)`);
    } else if (!allowedPorts.some((p) => Number(p) === port)) {
      violations.push(`port ${port} not in allowed ports ${JSON.stringify(allowedPorts)}`);
    }

    const allowedHosts = asList(caps["hosts"]);
    if (allowedHosts.length === 0) {
      violations.push(`host '${host}' blocked (deny-all, no hosts list)`);
    } else if (!allowedHosts.includes(host)) {
      violations.push(`host '${host}' not in allowed hosts ${JSON.stringify(allowedHosts)}`);
    }

    const allowedPaths = asList(caps["paths"]);
    if (allowedPaths.length === 0) {
      violations.push(`path '${path}' blocked (deny-all, no paths list)`);
    } else if (!allowedPaths.some((p) => globMatch(p, path))) {
      violations.push(`path '${path}' not in allowed paths ${JSON.stringify(allowedPaths)}`);
    }

    if (caps["require_auth"] === true) {
      if (auth === undefined || auth["type"] === undefined || auth["secret"] === undefined) {
        violations.push(
          "webhook requires auth config (type + secret) but none or incomplete provided",
        );
      }
    }

    return violations;
  }

  private _checkGlobField(value: string, capsKey: string): ReadonlyArray<string> {
    const caps = (this._wfCaps[capsKey] ?? {}) as Readonly<Record<string, unknown>>;
    const deny = asList(caps["deny"]);
    const allow = asList(caps["allow"]);

    for (const pattern of deny) {
      if (globMatch(pattern, value)) return [`'${value}' denied by pattern '${pattern}'`];
    }

    if (allow.length > 0) {
      if (!allow.some((p) => globMatch(p, value))) {
        return [`'${value}' not in allow list ${JSON.stringify(allow)}`];
      }
    } else {
      return [`'${value}' blocked (deny-all, no allow list)`];
    }

    return [];
  }

  private _checkAgent(runnerName: string): ReadonlyArray<string> {
    const allowed = asList(this._wfCaps["agents"]);
    if (allowed.length === 0) {
      return [`agent '${runnerName}' blocked (deny-all, no agents list)`];
    }
    if (!allowed.includes(runnerName)) {
      return [`agent '${runnerName}' not in allowed agents ${JSON.stringify(allowed)}`];
    }
    return [];
  }
}
