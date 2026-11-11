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
      { text: 'Explanation', link: '/explanation/' },
      { text: '9aia', link: 'https://9aia.com' },
    ],
    sidebar: [
      {
        text: 'Tutorials',
        items: [
          { text: 'Getting started', link: '/tutorials/getting-started' },
          { text: 'Your first gate', link: '/tutorials/first-gate' },
        ],
      },
      {
        text: 'How-to',
        items: [
          { text: 'Run a node', link: '/how-to/run-a-node' },
          { text: 'Run a hub', link: '/how-to/run-a-hub' },
          { text: 'Run a CI node', link: '/how-to/run-a-ci-node' },
          { text: 'CI runner fleet', link: '/how-to/ci-runner-fleet' },
          { text: 'Serve campaigns', link: '/how-to/serve-campaigns' },
        ],
      },
      {
        text: 'Reference',
        items: [
          { text: 'Event envelope', link: '/reference/event-envelope' },
          { text: 'on: event patterns', link: '/reference/on-grammar' },
          { text: 'Campaign schema', link: '/reference/campaign-schema' },
          { text: 'Scope resolution', link: '/reference/scope-resolution' },
          { text: 'Queue & supersede', link: '/reference/queue-supersede' },
          { text: 'Fleet runners', link: '/reference/fleet-runners' },
          { text: 'Distribution', link: '/reference/distribution' },
          { text: 'HTTP API', link: '/reference/api' },
          { text: 'Engine ops surface', link: '/reference/ops' },
          { text: 'Environment', link: '/reference/environment' },
        ],
      },
      {
        text: 'Explanation',
        items: [
          { text: 'Architecture', link: '/explanation/architecture' },
          { text: 'Event graph model', link: '/explanation/event-graph-model' },
          { text: 'Events', link: '/explanation/events' },
          { text: 'Abstraction via event types', link: '/explanation/abstraction' },
          { text: 'Drivers', link: '/explanation/drivers' },
          { text: 'Self-modifying workflows', link: '/explanation/self-modifying' },
          { text: 'Adapter ports', link: '/explanation/adapter-ports' },
          { text: 'Observers', link: '/explanation/observers' },
          { text: 'Stores', link: '/explanation/stores' },
          { text: 'Include & snippet resolution', link: '/explanation/include-resolution' },
          { text: 'Plugins & distribution', link: '/explanation/plugins' },
          { text: 'Resilience', link: '/explanation/resilience' },
          { text: 'Loop protection', link: '/explanation/loop-protection' },
          { text: 'Safety', link: '/explanation/safety' },
          { text: 'Threat model', link: '/explanation/threat-model' },
        ],
      },
    ],
    socialLinks: [{ icon: 'github', link: 'https://github.com/9aia/peregrin' }],
  },
})
