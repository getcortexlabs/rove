<script setup>
import { computed } from 'vue'
import { useData } from 'vitepress'

const labels = { 'pt-BR': 'estágio inicial', es: 'etapa inicial', 'zh-CN': '早期' }
const { lang } = useData()
const label = computed(() => labels[lang.value] || 'early')
</script>

<template>
  <span class="development-badge">{{ label }}</span>
</template>
