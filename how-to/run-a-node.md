# Run a node

`peregrin serve` binds `PEREGRIN_HOST:PEREGRIN_PORT` (default
127.0.0.1:8787). Non-loopback binds require `PEREGRIN_TOKEN`. The node
exposes the session plane plus campaign drivers over HTTP+SSE; point a
hub at it with `PEREGRIN_NODES`/`PEREGRIN_NODE_URL`.
