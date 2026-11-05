+++
id = "I-013"
title = ".handl/ vs .peregrin/ discovery root + auto-discovery boundaries for node-config workflows (security)"
status = "done"
created = "2026-11-04"
priority = "medium"
+++

# .handl/ vs .peregrin/ discovery root + auto-discovery boundaries for node-config workflows (security)

Status: **open** · Created: 2026-11-04

## Description

What is wrong and steps to reproduce.

## Checklist

- [ ] Action item

## Resolution

.peregrin/ added as a discovery root alongside .handl/ (preferred name, wins dedup; .handl kept for v1 compat). Implemented in peregrin-loader-yaml discover.rs + tests.
