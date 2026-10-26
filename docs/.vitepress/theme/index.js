import { h } from 'vue'
import DefaultTheme from 'vitepress/theme'
import DevelopmentBadge from './DevelopmentBadge.vue'
import OrbitSignature from './OrbitSignature.vue'
import './custom.css'
import './orbit-signature.css'

export default {
  extends: DefaultTheme,
  Layout: () => h(DefaultTheme.Layout, null, {
    'nav-bar-title-after': () => h(DevelopmentBadge),
    'layout-bottom': () => h(OrbitSignature)
  })
}
