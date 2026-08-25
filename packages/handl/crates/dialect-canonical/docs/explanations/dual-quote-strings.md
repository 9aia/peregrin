# Why both single and double quotes are accepted

handl string literals accept both `"..."` and `'...'`. The inner content is returned verbatim regardless of which quote character is used.

## Reason

handl expressions are embedded in YAML/JSON strings. YAML allows both quote styles at the host level. If handl only accepted one style, users would face quote conflicts: a YAML double-quoted string containing an handl expression with a double-quoted string literal would need escaping or different delimiters.

Accepting both styles lets users pick the one that doesn't conflict with the host format's quoting. No escaping needed — just use the other quote.

## No semantic difference

Both quote styles produce identical string values. There is no interpolation, no escape processing, no special behavior for either. The choice is purely syntactic convenience.
