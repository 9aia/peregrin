+++
id = "I-011"
title = "ship or drop .driver.yml presets (ADR-0026) + *.field.yml/dependencies (ADR-0032/33) — bundles assume them; never shipped"
status = "done"
created = "2026-11-04"
priority = "medium"
+++

# ship or drop .driver.yml presets (ADR-0026) + *.field.yml/dependencies (ADR-0032/33) — bundles assume them; never shipped

Status: **open** · Created: 2026-11-04

## Description

What is wrong and steps to reproduce.

## Checklist

- [ ] Action item

## Resolution

DROP. .field.yml/dependencies/.driver.yml presets were never shipped even in v1 (aspirational docs); no shipped bundle needs them (ci-gate proven e2e). .field.yml files stay in libraries/ as inert documentation of intended param surfaces.
