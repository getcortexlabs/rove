---
layout: home
title: Inferencia filogenética en Rust
description: Rove es un proyecto de inferencia filogenética en Rust, pensado primero para ARM. Empieza con un kernel de verosimilitud para datos morfológicos, verificado contra una referencia escalar.

hero:
  name: Rove
  text: Explora caminos evolutivos.
  tagline: Inferencia filogenética en Rust, pensada primero para ARM. Explora datos morfológicos en una aplicación nativa para Mac.
  image:
    src: /images/rove-logo.webp
    width: 640
    height: 640
    fetchpriority: high
    decoding: async
    alt: El logo de Rove, una letra R dibujada como nodos conectados.
  actions:
    - theme: brand
      text: Descargar para Mac
      link: https://getcortexlabs.com/download/rove
    - theme: alt
      text: ¿Qué es Rove?
      link: /es/guide/
    - theme: alt
      text: El núcleo de verosimilitud
      link: /es/guide/kernel
    - theme: alt
      text: GitHub
      link: https://github.com/getcortexlabs/rove

features:
  - title: Un kernel con la forma de los datos
    details: Las matrices morfológicas son en su mayoría caracteres binarios y ternarios. Rove dispone los parciales por patrones, de modo que el bucle interno recorre los patrones de sitio sea cual sea el número de estados.
    link: /es/guide/design
    linkText: El diseño
  - title: Mk, en forma cerrada
    details: El núcleo Mk hace la poda con la forma cerrada del modelo. Cada actualización de un hijo usa una suma ponderada por patrón en lugar de un producto de matrices k×k.
    link: /es/guide/kernel#ramas-en-mk
    linkText: Ramas en Mk
  - title: Verificado contra una referencia
    details: Una implementación recursiva simple se mantiene en el crate como referencia. Las pruebas de propiedades comparan el núcleo con ella en árboles aleatorios, con y sin reescalado.
    link: /es/guide/kernel
    linkText: El núcleo de verosimilitud
---

<div class="home-status">

::: info Etapa inicial
El núcleo de verosimilitud con la ruta directa de Mk y un kernel NEON, la corrección Mkv, las pruebas de igualdad y un muestreador independiente mínimo están implementados. El modelo de referencia con reloj relajado está implementado y comprobado en estados fijos. Las comparaciones de posterior cubren seis casos construidos con 20 taxones. El [benchmark ampliado](https://github.com/getcortexlabs/rove/blob/main/benchmarks/expanded/2026-10-30/README.md) cubre 20, 90 y 500 taxones y compara adaptadores de núcleo de BEAST/BEAGLE, separando tiempo y calidad del muestreo. Las ejecuciones mayores no cumplieron los criterios de convergencia. Una ruta AVX2 sigue pendiente. La app nativa para Mac se distribuye como imagen de disco firmada y notarizada para Apple silicon en [GitHub Releases](https://github.com/getcortexlabs/rove/releases/latest). [Ver lo que existe hoy →](/es/guide/#lo-que-existe-hoy)
:::

</div>

<div class="home-status">

## Hecho en colaboración

Rove nació de las conversaciones entre Rafael Augusto y **Thiago J. Silva**, el biólogo cuyas preguntas sobre análisis filogenético dieron origen al proyecto. Thiago trabaja junto a Rafael, aportando la perspectiva biológica, probando análisis y discutiendo el significado de los resultados.

</div>
