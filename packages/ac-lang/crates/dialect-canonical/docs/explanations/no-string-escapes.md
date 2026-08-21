# Why string literals have no escape sequences

String literals return their inner content verbatim. A backslash is a literal backslash, not an escape prefix.

## Reason

AC expressions are embedded in YAML/JSON/TOML strings — formats that already define their own escape rules. Adding a second escape layer inside AC string literals would create double-escaping confusion: `\n` in a YAML double-quoted string already means newline; if AC also interpreted `\n`, users would need `\\n` to get a literal backslash-n.

By keeping AC string literals raw, the host format's escaping is the only escaping. What you see between the quotes is what you get.

## What you do instead

- Need a newline in a string? Put it in the YAML/JSON value — the host format handles it.
- Need a literal quote inside a string? Use the other quote style: `"it's"` or `'say "hi"'`.
