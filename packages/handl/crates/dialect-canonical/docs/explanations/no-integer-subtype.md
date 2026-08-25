# Why there is no integer subtype

All numbers are IEEE 754 doubles. `42` and `42.0` are the same type. There is no separate integer type.

## Reason

handl values are JSON-compatible, and JSON has one number type. Introducing an integer subtype would create type-compatibility questions that JSON doesn't have: is `42` compatible with `42.0`? Can a `number` field accept an `integer` value and vice versa? Must every consumer of handl values handle two numeric types?

One number type eliminates all of these. The type system stays simpler, validation stays simpler, and interoperability with JSON ecosystems is exact.

## JSON Schema's `integer` keyword

JSON Schema's `integer` keyword is a validation constraint, not a separate type. A field with `type: integer` still holds a JSON number — it just rejects non-integer values at validation time. The type system maps `integer` to `Number` because the runtime value is a double.
