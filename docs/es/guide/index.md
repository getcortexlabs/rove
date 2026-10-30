---
description: "Rove es un proyecto de inferencia filogenética en Rust, pensado primero para ARM. Por qué empieza con un kernel de verosimilitud para datos morfológicos, qué existe hoy y qué no."
---

# ¿Qué es Rove?

Rove es un proyecto de inferencia filogenética escrito en Rust. Empieza con un kernel de verosimilitud para datos morfológicos, diseñado primero para CPUs ARM. El objetivo es la inferencia independiente: ejecutar un análisis desde una matriz de caracteres hasta una muestra de árboles sin depender de BEAST.

El nombre viene de explorar, recorrer y descubrir caminos.

::: info Estado del proyecto
Etapa inicial. El núcleo de verosimilitud con la ruta directa de Mk y un kernel NEON para ella, la corrección Mkv, el lector de NEXUS, la compresión de patrones, las pruebas numéricas y un muestreador independiente mínimo están implementados. Hay un experimento de referencia con BEAST 2.7.8 documentado. El [modelo de referencia](./reference-model) con reloj relajado está implementado y comprobado en estados fijos. Las comparaciones de posterior cubren seis casos construidos con 20 taxones. El [benchmark ampliado](https://github.com/getcortexlabs/rove/blob/main/benchmarks/expanded/2026-10-30/README.md) cubre 20, 90 y 500 taxones y compara adaptadores de núcleo de BEAST/BEAGLE, separando tiempo y calidad del muestreo. Las ejecuciones mayores no cumplieron los criterios de convergencia. Una ruta AVX2 sigue pendiente. El trabajo se sigue en los [issues](https://github.com/getcortexlabs/rove/issues).
:::

## Por qué otro kernel

Los espacios de estados pequeños y los alineamientos morfológicos cortos merecen medirse como una carga de trabajo propia.

- Mk tiene una forma cerrada, `P(t) = e·I + (1 − e)·π`, que permite una actualización O(k) por hijo y por patrón en lugar de una multiplicación de matrices O(k²) completa. El núcleo Mk de Rove hace la poda directamente con ella.
- La vectorización específica para ARM y la reutilización de cálculos repetidos son optimizaciones candidatas. Su valor requiere benchmarks comparables, que incluyan los costos de preparación y de actualización parcial.
- La referencia documentada con BEAST 2.7.8 usó su núcleo de verosimilitud en Java. Esa observación no establece el backend que usa cada versión, instalación o modelo de BEAST.

Rove parte de los datos: una disposición que vectoriza a lo largo de los patrones de sitio, y rutas rápidas para los modelos que usan los morfólogos. La página de [diseño](./design) recorre las decisiones.

## Lo que existe hoy

El motor es un único crate de Rust sin dependencias en tiempo de ejecución. Un paquete Swift
separado ofrece la [vista previa nativa para Mac](./macos-preview) para importar matrices, revisar ajustes, ejecutar análisis y seguir el progreso.

- `LikelihoodCore`: poda de Felsenstein sobre una disposición por patrones, con categorías de tasa, reescalado opcional por nodo y guardado/restauración del estado de los nodos para un muestreador. `LikelihoodCore::mk` hace la poda con la forma cerrada de Mk; `LikelihoodCore::new` multiplica una matriz de transición por rama.
- `Backend`: código portable para cualquier destino y, en aarch64, un kernel NEON para la actualización de nodo de Mk que da los mismos resultados. Ver [Backends](./kernel#backends).
- `model::Mk`: el modelo Mk con su matriz de transición en forma cerrada, para frecuencias de estados uniformes o personalizadas, que se validan.
- `ascertainment::VariableCharacters`: la corrección Mkv para matrices de caracteres variables, comprobada contra dos estados fijos de BEAST 2.7.8.
- `ascertainment::InformativeProbability`: condicionamiento Mkparsinf con caché para Mk no ordenado de frecuencias iguales, comprobado mediante enumeración completa de patrones y BEAST 2.7.8. Consulta [selección de caracteres](./character-selection).
- `reference::Problem`: una implementación recursiva simple que se conserva como referencia en las pruebas.
- `rates::GammaRates`, `rates::RelaxedClock`: [Mk + gamma, lognormal](./reference-model).
- `mcmc::Sampler` y el comando `rove sample`: un muestreador de Metropolis–Hastings para caracteres binarios y multiestado bajo Mk o Mkv particionado, un reloj estricto o relajado lognormal y un prior de nacimiento–muerte para el árbol, con restricciones de monofilia. Ver [Muestreo de árboles](./sampler).
- `nexus::parse` y `data::Matrix::compress`: entrada morfológica, observaciones preservadas, particiones explícitas y patrones de sitio ponderados. Ver [Lectura de NEXUS](./nexus).
- Pruebas de propiedades que comparan los dos tipos de núcleo con la referencia en árboles aleatorios de 2 a 10 terminales, 2 a 6 estados y 1 a 4 categorías de tasa, incluidas longitudes de rama de 10⁻⁹ a 100 y secuencias de cambios aceptados y rechazados.
- Una matriz sintética de control positivo de 20 taxones y 50 caracteres binarios, con el script que la genera y variantes degradadas.

La página del [núcleo de verosimilitud](./kernel) muestra cómo llamarlo. La página de [matrices sintéticas](./fixtures) describe el control.

## Lo que aún no existe

- Modelos de particiones definidos por el usuario, fechas de terminales, calibraciones y otros priors de árbol.
- Una ruta de código AVX2. Los destinos que no son aarch64 ejecutan el código portable.
- Adaptadores de producción para BEAST 2 o BEAGLE. Los adaptadores de benchmark son herramientas separadas.

Nada de esta lista se documenta como disponible en ningún lugar de este sitio. El [roadmap](./roadmap) indica el orden.

## BEAST, como referencia

BEAST no es una dependencia. Es la referencia para validación y benchmarking: primero se comparan verosimilitudes en estados fijos, después ejecuciones de inferencia completas. BEAST 2 y BEAST X son bases de código distintas, y los resultados de una no establecen los de la otra, así que las comparaciones nombran la implementación y la versión contra las que se ejecutaron.
