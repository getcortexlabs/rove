---
layout: home
title: Inferência filogenética em Rust
description: O Rove é um projeto de inferência filogenética em Rust, pensado primeiro para ARM. Começa com um kernel de verossimilhança para dados morfológicos, verificado contra uma referência escalar.

hero:
  name: Rove
  text: Explore caminhos evolutivos.
  tagline: Inferência filogenética em Rust, pensada primeiro para ARM. Explore dados morfológicos em um aplicativo nativo para Mac.
  image:
    src: /images/rove-logo.webp
    width: 640
    height: 640
    fetchpriority: high
    decoding: async
    alt: O logotipo do Rove, uma letra R desenhada como nós conectados.
  actions:
    - theme: brand
      text: Baixar para Mac
      link: https://github.com/getcortexlabs/rove/releases/latest
    - theme: alt
      text: O que é o Rove?
      link: /pt-BR/guide/
    - theme: alt
      text: O núcleo de verossimilhança
      link: /pt-BR/guide/kernel
    - theme: alt
      text: GitHub
      link: https://github.com/getcortexlabs/rove

features:
  - title: Um kernel moldado para os dados
    details: Matrizes morfológicas são, em sua maioria, caracteres binários e ternários. O Rove dispõe as parciais em ordem pattern-major, de modo que o laço interno percorre os padrões de sítio seja qual for o número de estados.
    link: /pt-BR/guide/design
    linkText: O design
  - title: Mk, em forma fechada
    details: O núcleo Mk faz a poda com a forma fechada do modelo. Cada atualização de filho usa uma soma ponderada por padrão em vez de um produto de matrizes k×k.
    link: /pt-BR/guide/kernel#ramos-no-mk
    linkText: Ramos no Mk
  - title: Verificado contra uma referência
    details: Uma implementação recursiva simples permanece na crate como referência. Testes de propriedade comparam o núcleo com ela em árvores aleatórias, com e sem reescalonamento.
    link: /pt-BR/guide/kernel
    linkText: O núcleo de verossimilhança
---

<div class="home-status">

::: info Estágio inicial
O núcleo de verossimilhança com o caminho direto do Mk e um kernel NEON, a correção Mkv, os testes de igualdade e um amostrador independente mínimo estão implementados. O modelo de referência com relógio relaxado está implementado e conferido em estados fixos. Comparações de posterior cobrem seis casos construídos com 20 táxons. O [benchmark ampliado](https://github.com/getcortexlabs/rove/blob/main/benchmarks/expanded/2026-10-30/README.md) cobre 20, 90 e 500 táxons e compara adaptadores de núcleo do BEAST/BEAGLE, separando tempo e qualidade da amostragem. As execuções maiores não atingiram os critérios de convergência. Um caminho AVX2 continua como trabalho futuro. O app nativo para Mac é distribuído como imagem de disco assinada e notarizada para Apple silicon no [GitHub Releases](https://github.com/getcortexlabs/rove/releases/latest). [Veja o que existe hoje →](/pt-BR/guide/#o-que-existe-hoje)
:::

</div>

<div class="home-status">

## Feito em parceria

O Rove nasceu das conversas entre Rafael Augusto e **Thiago J. Silva**, o biólogo cujas perguntas sobre análise filogenética deram início ao projeto. Thiago trabalha junto com Rafael, trazendo o olhar da biologia, testando análises e discutindo o significado dos resultados.

</div>
