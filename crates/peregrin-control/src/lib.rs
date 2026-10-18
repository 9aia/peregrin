//! peregrin-control — the policy layer over sepia planes: which actor
//! may hold a session, when a takeover preempts, merged multi-node
//! session views, routing ops to the owning node, offline ordering.
//! Mechanism (attach modes, probes) stays in `sepia-session` — this
//! crate decides, sepia executes.
