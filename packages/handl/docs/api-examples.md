# Runtime API Examples

This page demonstrates how to use some runtime APIs.

## `useData` Composable

The `useData` composable is available on every page.

```vue
<script setup>
import { useData } from "vitepress";

const { theme, page, frontmatter } = useData();
</script>
```

## `useRoute` Composable

The `useRoute` composable is available on every page.

```vue
<script setup>
import { useRoute } from "vitepress";

const route = useRoute();
</script>
```

## More

Check out the [VitePress Runtime API Reference](https://vitepress.dev/reference/runtime-api) for more.
