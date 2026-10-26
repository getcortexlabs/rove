import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'Rove',
  description: 'Rove documentation.',
  lang: 'en-US',
  // Keep builds independent of Git history, including source archives.
  lastUpdated: false,
  themeConfig: {
    logo: { src: '/rove-logo.png', alt: 'Rove logo' },
    sidebar: false,
    outline: false,
    socialLinks: [{ icon: 'github', link: 'https://github.com/rafaelaugustos/rove' }],
  },
})
