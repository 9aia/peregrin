import { defineConfig } from "vitepress";

export default defineConfig({
  title: "Peregrin",
  description: "Vite+ monorepo documentation",
  base: "/",
  themeConfig: {
    nav: [{ text: "Home", link: "/" }],
    sidebar: [
      {
        text: "Guide",
        items: [{ text: "Getting Started", link: "/getting-started" }],
      },
    ],
    socialLinks: [{ icon: "github", link: "https://github.com/9aia/peregrin" }],
  },
});
