---
layout: home
title: Inferência filogenética em Rust
description: O Rove é um projeto de inferência filogenética em Rust, pensado primeiro para ARM. Começa com um kernel de verossimilhança para dados morfológicos, verificado contra uma referência escalar.

hero:
  name: Rove
  text: Explore caminhos evolutivos.
  tagline: Inferência filogenética em Rust, construída primeiro para ARM. Hoje, um kernel de verossimilhança para dados morfológicos. Em seguida, análise independente.
  image:
    src: /images/rove-logo.webp
    width: 640
    height: 640
    fetchpriority: high
    decoding: async
    alt: O logotipo do Rove, uma letra R desenhada como nós conectados.
  actions:
    - theme: brand
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
    details: O modelo Mk vem com sua matriz de transição em forma fechada. Um caminho rápido que dispensa o produto genérico k×k está no roadmap.
    link: /pt-BR/guide/roadmap
    linkText: O roadmap
  - title: Verificado contra uma referência
    details: Uma implementação recursiva simples permanece na crate como referência. Testes de propriedade comparam o núcleo com ela em árvores aleatórias, com e sem reescalonamento.
    link: /pt-BR/guide/kernel
    linkText: O núcleo de verossimilhança
---

<div class="home-status">

::: info Estágio inicial
O núcleo de verossimilhança, o modelo Mk com a correção Mkv e os testes de igualdade estão implementados. Inferência independente, caminhos SIMD e comparações entre motores não estão. [Veja o que existe hoje →](/pt-BR/guide/#o-que-existe-hoje)
:::

</div>
