import { defineConfig } from 'vitepress'
import { existsSync } from 'node:fs'

const site = 'https://rove.getcortexlabs.com'
const repo = 'https://github.com/getcortexlabs/rove'
const socialImage = `${site}/images/rove-social.jpg`
const socialImageAlt = 'Rove. Explore evolutionary paths. Phylogenetic inference in Rust, built ARM-first, with the Rove emblem over a glowing phylogenetic tree.'
const cortex = '<a href="https://getcortexlabs.com/">Cortex Labs</a>'

const locales = {
  root: {
    dir: '',
    lang: 'en-US',
    hreflang: 'en',
    ogLocale: 'en_US',
    label: 'English',
    description: 'ARM-first phylogenetic inference in Rust. Starting with a likelihood kernel for morphological data, checked against a scalar reference.',
    ui: {
      guide: 'Guide',
      roadmap: 'Roadmap',
      project: 'The project',
      code: 'The code',
      what: 'What is Rove?',
      macPreview: 'Mac preview',
      design: 'Design',
      kernel: 'The likelihood core',
      sampler: 'Sampling trees',
      referenceModel: 'The reference model',
      nexus: 'Reading NEXUS',
      fixtures: 'Synthetic matrices',
      outline: 'On this page',
      edit: 'Edit this page on GitHub',
      message: 'Phylogenetic inference in Rust. MIT or Apache-2.0.',
      copyright: `By Rafael Augusto and Thiago J. Silva · ${cortex}`
    }
  },
  'pt-BR': {
    dir: 'pt-BR',
    lang: 'pt-BR',
    hreflang: 'pt-BR',
    ogLocale: 'pt_BR',
    label: 'Português',
    description: 'Inferência filogenética em Rust, pensada primeiro para ARM. Começa por um kernel de verossimilhança para dados morfológicos, conferido contra uma referência escalar.',
    ui: {
      guide: 'Guia',
      roadmap: 'Roadmap',
      project: 'O projeto',
      code: 'O código',
      what: 'O que é o Rove?',
      macPreview: 'Prévia para Mac',
      design: 'Design',
      kernel: 'O núcleo de verossimilhança',
      sampler: 'Amostrando árvores',
      referenceModel: 'O modelo de referência',
      nexus: 'Lendo NEXUS',
      fixtures: 'Matrizes sintéticas',
      outline: 'Nesta página',
      edit: 'Editar esta página no GitHub',
      message: 'Inferência filogenética em Rust. MIT ou Apache-2.0.',
      copyright: `Por Rafael Augusto e Thiago J. Silva · ${cortex}`,
      docFooter: { prev: 'Anterior', next: 'Próxima' },
      darkModeSwitchLabel: 'Aparência',
      lightModeSwitchTitle: 'Mudar para o tema claro',
      darkModeSwitchTitle: 'Mudar para o tema escuro',
      sidebarMenuLabel: 'Menu',
      returnToTopLabel: 'Voltar ao topo',
      langMenuLabel: 'Mudar o idioma',
      notFound: { title: 'Página não encontrada', quote: 'Esta página ainda não está aqui.', linkText: 'Voltar ao início' }
    },
    search: {
      button: { buttonText: 'Buscar', buttonAriaLabel: 'Buscar na documentação' },
      modal: {
        noResultsText: 'Nenhum resultado para',
        resetButtonTitle: 'Limpar busca',
        displayDetails: 'Exibir detalhes',
        footer: { selectText: 'selecionar', navigateText: 'navegar', closeText: 'fechar' }
      }
    }
  },
  es: {
    dir: 'es',
    lang: 'es',
    hreflang: 'es',
    ogLocale: 'es_ES',
    label: 'Español',
    description: 'Inferencia filogenética en Rust, pensada primero para ARM. Empieza por un kernel de verosimilitud para datos morfológicos, verificado contra una referencia escalar.',
    ui: {
      guide: 'Guía',
      roadmap: 'Roadmap',
      project: 'El proyecto',
      code: 'El código',
      what: '¿Qué es Rove?',
      macPreview: 'Vista previa para Mac',
      design: 'Diseño',
      kernel: 'El núcleo de verosimilitud',
      sampler: 'Muestreo de árboles',
      referenceModel: 'El modelo de referencia',
      nexus: 'Lectura de NEXUS',
      fixtures: 'Matrices sintéticas',
      outline: 'En esta página',
      edit: 'Editar esta página en GitHub',
      message: 'Inferencia filogenética en Rust. MIT o Apache-2.0.',
      copyright: `Por Rafael Augusto y Thiago J. Silva · ${cortex}`,
      docFooter: { prev: 'Anterior', next: 'Siguiente' },
      darkModeSwitchLabel: 'Apariencia',
      lightModeSwitchTitle: 'Cambiar al tema claro',
      darkModeSwitchTitle: 'Cambiar al tema oscuro',
      sidebarMenuLabel: 'Menú',
      returnToTopLabel: 'Volver arriba',
      langMenuLabel: 'Cambiar idioma',
      notFound: { title: 'Página no encontrada', quote: 'Esta página todavía no existe.', linkText: 'Volver al inicio' }
    },
    search: {
      button: { buttonText: 'Buscar', buttonAriaLabel: 'Buscar en la documentación' },
      modal: {
        noResultsText: 'No hay resultados para',
        resetButtonTitle: 'Borrar búsqueda',
        displayDetails: 'Mostrar detalles',
        footer: { selectText: 'seleccionar', navigateText: 'navegar', closeText: 'cerrar' }
      }
    }
  },
  'zh-CN': {
    dir: 'zh-CN',
    lang: 'zh-CN',
    hreflang: 'zh-CN',
    ogLocale: 'zh_CN',
    label: '简体中文',
    description: '用 Rust 编写、优先面向 ARM 的系统发育推断。从针对形态学数据的似然内核起步，并与标量参考实现相互校验。',
    ui: {
      guide: '指南',
      roadmap: '路线图',
      project: '项目',
      code: '代码',
      what: 'Rove 是什么？',
      macPreview: 'Mac 预览版',
      design: '设计',
      kernel: '似然核心',
      sampler: '树的采样',
      referenceModel: '参考模型',
      nexus: '读取 NEXUS',
      fixtures: '合成矩阵',
      outline: '本页目录',
      edit: '在 GitHub 上编辑此页',
      message: '用 Rust 实现的系统发育推断。MIT 或 Apache-2.0 许可。',
      copyright: `由 Rafael Augusto 与 Thiago J. Silva 共同打造 · ${cortex}`,
      docFooter: { prev: '上一页', next: '下一页' },
      darkModeSwitchLabel: '外观',
      lightModeSwitchTitle: '切换为浅色主题',
      darkModeSwitchTitle: '切换为深色主题',
      sidebarMenuLabel: '菜单',
      returnToTopLabel: '返回顶部',
      langMenuLabel: '切换语言',
      notFound: { title: '页面未找到', quote: '此页面尚不存在。', linkText: '返回首页' }
    },
    search: {
      button: { buttonText: '搜索', buttonAriaLabel: '搜索文档' },
      modal: {
        noResultsText: '没有找到相关结果',
        resetButtonTitle: '清空搜索',
        displayDetails: '显示详情',
        footer: { selectText: '选择', navigateText: '切换', closeText: '关闭' }
      }
    }
  }
}

function themeFor(locale) {
  const prefix = locale.dir ? `/${locale.dir}` : ''
  const { ui } = locale
  const { guide, roadmap, project, code, what, macPreview, design, kernel, sampler, referenceModel, nexus, fixtures, outline, edit, message, copyright, ...labels } = ui
  return {
    nav: [
      { text: guide, link: `${prefix}/guide/`, activeMatch: `${prefix}/guide/(?!roadmap)` },
      { text: roadmap, link: `${prefix}/guide/roadmap` }
    ],
    sidebar: {
      [`${prefix}/guide/`]: [
        {
          text: project,
          items: [
            { text: what, link: `${prefix}/guide/` },
            { text: macPreview, link: `${prefix}/guide/macos-preview` },
            { text: design, link: `${prefix}/guide/design` },
            { text: roadmap, link: `${prefix}/guide/roadmap` }
          ]
        },
        {
          text: code,
          items: [
            { text: kernel, link: `${prefix}/guide/kernel` },
            { text: sampler, link: `${prefix}/guide/sampler` },
            { text: referenceModel, link: `${prefix}/guide/reference-model` },
            { text: nexus, link: `${prefix}/guide/nexus` },
            { text: fixtures, link: `${prefix}/guide/fixtures` }
          ]
        }
      ]
    },
    outline: { level: [2, 3], label: outline },
    editLink: { pattern: `${repo}/edit/main/docs/:path`, text: edit },
    footer: { message, copyright },
    ...labels
  }
}

function localeOf(relativePath) {
  const dir = relativePath.split('/')[0]
  return Object.values(locales).find(locale => locale.dir && locale.dir === dir) || locales.root
}

function pageUrl(dir, sourcePath) {
  const path = sourcePath.replace(/index\.md$/, '').replace(/\.md$/, '')
  return dir ? `${site}/${dir}/${path}` : `${site}/${path}`
}

export default defineConfig({
  title: 'Rove',
  description: locales.root.description,
  lang: locales.root.lang,
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
    const locale = localeOf(pageData.relativePath)
    const sourcePath = locale.dir ? pageData.relativePath.slice(locale.dir.length + 1) : pageData.relativePath
    const url = pageUrl(locale.dir, sourcePath)
    const title = pageData.title ? `${pageData.title} | Rove` : 'Rove'
    const description = pageData.description || locale.description
    const alternates = Object.values(locales)
      .filter(other => existsSync(new URL(`../${other.dir ? `${other.dir}/` : ''}${sourcePath}`, import.meta.url)))
    const structuredData = {
      '@context': 'https://schema.org',
      '@graph': [
        {
          '@type': 'WebSite',
          '@id': `${site}/#website`,
          name: 'Rove',
          url: `${site}/`,
          inLanguage: Object.values(locales).map(other => other.hreflang)
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
          description,
          inLanguage: locale.hreflang,
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
      ...alternates.map(other => ['link', { rel: 'alternate', hreflang: other.hreflang, href: pageUrl(other.dir, sourcePath) }]),
      ['link', { rel: 'alternate', hreflang: 'x-default', href: pageUrl('', sourcePath) }],
      ['meta', { name: 'robots', content: 'index, follow, max-image-preview:large' }],
      ['meta', { property: 'og:url', content: url }],
      ['meta', { property: 'og:locale', content: locale.ogLocale }],
      ...alternates.filter(other => other !== locale).map(other => ['meta', { property: 'og:locale:alternate', content: other.ogLocale }]),
      ['meta', { property: 'og:title', content: title }],
      ['meta', { property: 'og:description', content: description }],
      ['meta', { name: 'twitter:title', content: title }],
      ['meta', { name: 'twitter:description', content: description }],
      ['script', { type: 'application/ld+json' }, JSON.stringify(structuredData).replace(/</g, '\\u003c')]
    ]
  },
  themeConfig: {
    logo: { src: '/logo.webp', alt: '' },
    socialLinks: [{ icon: 'github', link: repo }],
    search: {
      provider: 'local',
      options: {
        miniSearch: {
          options: {
            // Segment with the same rules while indexing and querying, so a
            // Chinese paragraph does not become one unsearchable token.
            tokenize: text =>
              Array.from(new Intl.Segmenter('zh-CN', { granularity: 'word' }).segment(text))
                .filter(part => part.isWordLike)
                .map(part => part.segment)
          }
        },
        locales: Object.fromEntries(
          Object.entries(locales)
            .filter(([, locale]) => locale.search)
            .map(([key, locale]) => [key, { translations: locale.search }])
        )
      }
    }
  },
  locales: Object.fromEntries(
    Object.entries(locales).map(([key, locale]) => [
      key,
      {
        label: locale.label,
        lang: locale.lang,
        link: locale.dir ? `/${locale.dir}/` : '/',
        description: locale.description,
        themeConfig: themeFor(locale)
      }
    ])
  )
})
