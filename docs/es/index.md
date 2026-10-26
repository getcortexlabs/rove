---
layout: home
title: Inferencia filogenética en Rust
description: Rove es un proyecto de inferencia filogenética en Rust, pensado primero para ARM. Empieza con un kernel de verosimilitud para datos morfológicos, verificado contra una referencia escalar.

hero:
  name: Rove
  text: Explora caminos evolutivos.
  tagline: Inferencia filogenética en Rust, construida primero para ARM. Hoy, un kernel de verosimilitud para datos morfológicos. Después, análisis independiente.
  image:
    src: /images/rove-logo.webp
    width: 640
    height: 640
    fetchpriority: high
    decoding: async
    alt: El logo de Rove, una letra R dibujada como nodos conectados.
  actions:
    - theme: brand
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
    details: El modelo Mk incluye su matriz de transición en forma cerrada. Una ruta rápida que evita el producto genérico k×k está en el roadmap.
    link: /es/guide/roadmap
    linkText: El roadmap
  - title: Verificado contra una referencia
    details: Una implementación recursiva simple se mantiene en el crate como referencia. Las pruebas de propiedades comparan el núcleo con ella en árboles aleatorios, con y sin reescalado.
    link: /es/guide/kernel
    linkText: El núcleo de verosimilitud
---

<div class="home-status">

::: info Etapa inicial
El núcleo de verosimilitud, el modelo Mk y las pruebas de igualdad están implementados. La inferencia independiente, las rutas SIMD y los benchmarks no. [Ver lo que existe hoy →](/es/guide/#lo-que-existe-hoy)
:::

</div>
