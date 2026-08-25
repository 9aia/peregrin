import { defineConfig } from "vitepress";

// https://vitepress.dev/reference/site-config
export default defineConfig({
  title: "handl",
  description: "Documentation for handl",
  // Existing docs reference ./DIALECT which is not present yet
  ignoreDeadLinks: true,
  themeConfig: {
    // https://vitepress.dev/reference/default-theme-config
    nav: [
      { text: "Home", link: "/" },
      { text: "Examples", link: "/markdown-examples" },
      { text: "API", link: "/api/" },
      { text: "Reference", link: "/reference/" },
    ],

    sidebar: [
      {
        text: "Examples",
        items: [
          { text: "Markdown Examples", link: "/markdown-examples" },
          { text: "Runtime API Examples", link: "/api-examples" },
        ],
      },
      {
        text: "API",
        items: [{ text: "Crate Index", link: "/api/" }],
      },
      {
        text: "Reference",
        items: [
          { text: "Overview", link: "/reference/" },
          { text: "Math", link: "/reference/math" },
          { text: "String", link: "/reference/str" },
          { text: "Array", link: "/reference/array" },
          { text: "Object", link: "/reference/obj" },
          { text: "Type", link: "/reference/type" },
          { text: "Bytes", link: "/reference/bytes" },
          { text: "Boolean", link: "/reference/bool" },
          { text: "Compare", link: "/reference/cmp" },
          { text: "Function", link: "/reference/fn" },
          { text: "Mood", link: "/reference/mood" },
          { text: "Option", link: "/reference/option" },
          { text: "Result", link: "/reference/result" },
          { text: "Set", link: "/reference/set" },
        ],
      },
    ],

    socialLinks: [{ icon: "github", link: "https://github.com/9aia/peregrin" }],
  },
});
