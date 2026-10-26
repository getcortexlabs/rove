import { defineConfig } from 'vitepress'

const site = 'https://rove.getcortexlabs.com'
const repo = 'https://github.com/getcortexlabs/rove'
const description = 'ARM-first phylogenetic inference in Rust. Starting with a likelihood kernel for morphological data, checked against a scalar reference.'
const socialImage = `${site}/images/rove-social.jpg`
const socialImageAlt = 'Rove. Explore evolutionary paths. Phylogenetic inference in Rust, built ARM-first, with the Rove emblem over a glowing phylogenetic tree.'

export default defineConfig({
  title: 'Rove',
  description,
  lang: 'en-US',
  cleanUrls: true,
  appearance: true,
  // Keep builds independent of Git history, including source archives.
  lastUpdated: false,
  sitemap: { hostname: site },
  head: [
    ['link', { rel: 'icon', type: 'image/png', href: '/favicon.png' }],
    ['link', { rel: 'apple-touch-icon', href: '/apple-touch-icon.png' }],
    ['meta', { name: 'theme-color', content: '#2f5fe6' }],
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { property: 'og:site_name', content: 'Rove' }],
    ['meta', { property: 'og:locale', content: 'en_US' }],
    ['meta', { property: 'og:image', content: socialImage }],
    ['meta', { property: 'og:image:type', content: 'image/jpeg' }],
    ['meta', { property: 'og:image:width', content: '1200' }],
    ['meta', { property: 'og:image:height', content: '630' }],
    ['meta', { property: 'og:image:alt', content: socialImageAlt }],
    ['meta', { name: 'twitter:card', content: 'summary_large_image' }],
    ['meta', { name: 'twitter:image', content: socialImage }],
    ['meta', { name: 'twitter:image:alt', content: socialImageAlt }]
  ],
  transformHead({ pageData }) {
    if (pageData.isNotFound || pageData.relativePath === '404.md') {
      return [['meta', { name: 'robots', content: 'noindex, follow' }]]
    }
    const path = pageData.relativePath.replace(/index\.md$/, '').replace(/\.md$/, '')
    const url = `${site}/${path}`
    const title = pageData.title ? `${pageData.title} | Rove` : 'Rove'
    const pageDescription = pageData.description || description
    const structuredData = {
      '@context': 'https://schema.org',
      '@graph': [
        {
          '@type': 'WebSite',
          '@id': `${site}/#website`,
          name: 'Rove',
          url: `${site}/`,
          inLanguage: 'en'
        },
        {
          '@type': 'SoftwareSourceCode',
          '@id': `${site}/#code`,
          name: 'Rove',
          codeRepository: repo,
          programmingLanguage: 'Rust',
          license: 'https://spdx.org/licenses/MIT.html',
          url: `${site}/`
        },
        {
          '@type': 'WebPage',
          '@id': `${url}#webpage`,
          url,
          name: title,
          description: pageDescription,
          inLanguage: 'en',
          isPartOf: { '@id': `${site}/#website` },
          primaryImageOfPage: {
            '@type': 'ImageObject',
            contentUrl: socialImage,
            width: 1200,
            height: 630
          }
        }
      ]
    }
    return [
      ['link', { rel: 'canonical', href: url }],
      ['meta', { name: 'robots', content: 'index, follow, max-image-preview:large' }],
      ['meta', { property: 'og:url', content: url }],
      ['meta', { property: 'og:title', content: title }],
      ['meta', { property: 'og:description', content: pageDescription }],
      ['meta', { name: 'twitter:title', content: title }],
      ['meta', { name: 'twitter:description', content: pageDescription }],
      ['script', { type: 'application/ld+json' }, JSON.stringify(structuredData).replace(/</g, '\\u003c')]
    ]
  },
  themeConfig: {
    logo: { src: '/logo.webp', alt: '' },
    nav: [
      { text: 'Guide', link: '/guide/', activeMatch: '/guide/(?!roadmap)' },
      { text: 'Roadmap', link: '/guide/roadmap' }
    ],
    sidebar: {
      '/guide/': [
        {
          text: 'The project',
          items: [
            { text: 'What is Rove?', link: '/guide/' },
            { text: 'Design', link: '/guide/design' },
            { text: 'Roadmap', link: '/guide/roadmap' }
          ]
        },
        {
          text: 'The code',
          items: [
            { text: 'The likelihood core', link: '/guide/kernel' },
            { text: 'Reading NEXUS', link: '/guide/nexus' },
            { text: 'Synthetic matrices', link: '/guide/fixtures' }
          ]
        }
      ]
    },
    outline: { level: [2, 3], label: 'On this page' },
    socialLinks: [{ icon: 'github', link: repo }],
    search: { provider: 'local' },
    editLink: {
      pattern: `${repo}/edit/main/docs/:path`,
      text: 'Edit this page on GitHub'
    },
    footer: {
      message: 'Phylogenetic inference in Rust. MIT or Apache-2.0.',
      copyright: 'Maintained by Rafael Augusto and <a href="https://getcortexlabs.com/">Cortex Labs</a>'
    }
  }
})
