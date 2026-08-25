# Naming: "fields" not "keys"

The reserved `$:` key declares typed, validated members of a document. The term "fields" is deliberate:

- **Fields imply schema.** A field has a name, a type, and constraints. "Keys" is generic — every object property in JSON/YAML is already a key.
- **Self-reference breaks.** `$:` is a reserved key that cannot be declared as a field name. Renaming to `keys:` yields "keys is a reserved key that cannot be declared as a key name" — the sentence collapses.
- **Convention.** Schema languages (protobuf, Avro, SQL, JSON Schema's "properties") use "fields" or "properties" for typed, declared members.
