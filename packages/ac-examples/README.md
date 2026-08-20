# ac-examples

Example workflows and reusable fragments for the Agent Campaigns (Peregrin)
TypeScript runtime. These YAMLs were ported from the original Python
`agent-campaign` reference implementation.

## Layout

- `examples/` — runnable example workflows.
- `workflows/` — meta-workflows (e.g. `create-workflow`).

## Running

```bash
ac run packages/ac-examples/examples/review-pr.yaml
ac run packages/ac-examples/workflows/create-workflow/main.yaml
```

## Notes

- `capability-confinement` and the reusable `lib/` nodes rely on pure data
  where possible. Example-specific Python handlers have been omitted in favor
  of the TypeScript `python` driver (`params.script` / `params.module` / `params.function`).
- Real campaign data under the original `campaigns/` directory was not
  ported because it contains proprietary workflows.
