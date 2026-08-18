# Comments

Rules for comments and JSDoc across the workspace.

## Universal rule

- Comment why, not what. Code that needs a "what" comment should be rewritten.
- Every exported function, type, class, and constant gets a JSDoc / TSDoc comment.
- JSDoc should describe intent, invariants, and failure modes.

## JSDoc

- Use `@param` and `@returns` when the meaning is not obvious from types.
- Document error cases: `@throws` is not used; document the `Effect` error type in the description.
- Keep comments one sentence per line when possible.

## Inline comments

- Use inline comments for non-obvious domain decisions and workarounds.
- Don't state the obvious. `// increment i` is noise.
- Don't comment out code — delete it.
