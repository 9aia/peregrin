import fs from "node:fs";

const _haveRlimitAs = (): boolean => false;

export function makePreexec(
  memLimitMb: number | null = null,
  oomScoreAdj: number | null = null,
): (() => void) | null {
  const wantRlimit = memLimitMb !== null && _haveRlimitAs();
  const wantOom = oomScoreAdj !== null && process.platform === "linux";
  if (!wantRlimit && !wantOom) return null;

  const oom = oomScoreAdj === null ? null : Number(oomScoreAdj);

  return () => {
    if (oom !== null) {
      try {
        fs.writeFileSync("/proc/self/oom_score_adj", String(oom));
      } catch {}
    }
  };
}
