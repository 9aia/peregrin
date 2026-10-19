import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'peregrin',
  description: 'Event-driven orchestration for agent workflows — nodes, campaigns, remote transport.',
  base: '/peregrin/',
  themeConfig: {
    nav: [
      { text: 'Tutorials', link: '/tutorials/getting-started' },
      { text: 'How-to', link: '/how-to/' },
      { text: 'Reference', link: '/reference/' },
      { text: 'Explanation', link: '/explanation/architecture' },
    ],
    sidebar: [
      {
        text: 'Tutorials',
        items: [{ text: 'Getting started', link: '/tutorials/getting-started' }],
      },
      {
        text: 'How-to',
        items: [{ text: 'Run a node', link: '/how-to/run-a-node' }],
      },
      {
        text: 'Reference',
        items: [
          { text: 'Environment', link: '/reference/environment' },
          { text: 'Crate API docs', link: '/api/' },
        ],
      },
      {
        text: 'Explanation',
        items: [{ text: 'Architecture', link: '/explanation/architecture' }],
      },
    ],
    socialLinks: [{ icon: 'github', link: 'https://github.com/9aia/peregrin' }],
  },
})
