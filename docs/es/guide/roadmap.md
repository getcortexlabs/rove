---
description: "El orden de implementación de Rove: establecer la corrección y líneas base medidas, después optimizar el kernel y construir la inferencia independiente."
---

# Roadmap

Rove está en una etapa inicial. No hay crate publicado ni fechas. Esto es el orden de implementación, no un calendario.

| Paso | Estado |
| --- | --- |
| Núcleo de verosimilitud escalar y pruebas numéricas de referencia | Hecho |
| CI en ARM64, x86_64 y la versión mínima de Rust | Hecho |
| Análisis de referencia reproducible con BEAST 2.7.8 | Hecho |
| NEXUS morfológico y compresión de patrones observados | Hecho |
| Línea base del núcleo Rust | Hecho, #8 |
| Benchmarks de inferencia completa y entre motores | Ejecuciones con 20/90/500 taxones y adaptadores de núcleo medidos; límites del muestreo registrados, #8 |
| Corrección de ascertainment de Mkv | Hecho |
| Poda directa de Mk | Hecho |
| MCMC independiente mínimo | Hecho |
| Vectorización NEON | Hecho |
| Modelo de referencia con reloj relajado | Hecho, #16 |
| Validación de inferencia completa | Hecho en seis casos de 20 taxones, #12 |
| Eficiencia del muestreo con reloj relajado | Evaluada; movimientos conjuntos opcionales, se mantiene el predeterminado clásico, #27 |
| Vista previa nativa para importar y preparar análisis | Implementado, #28 |
| Ejecución y progreso en la aplicación | Implementado, #29 |
| Historial de ejecuciones y reutilización de ajustes | Implementado, #30 |
| Selección explícita del grupo externo y restricciones aplicadas | Implementado, #31 |
| Resumen MCC posterior y exportación anotada | Implementado, #32 |
| Explorador nativo de árboles anotados y figuras SVG | Implementado, #33 |
| Diagnósticos de muestreo e inspección del burn-in en la app | Implementado, #34 |
| Cadenas en paralelo e informes de convergencia | Implementado, #23 |
| Reescalado por potencias de dos | Implementado, #18 |
| Checkpoints y reanudación de ejecuciones | Implementado en CLI y History nativo, #22 |
| Particiones por número de estados | Implementado, #19 |
| Condicionamiento a caracteres informativos para parsimonia | Implementado, #20 |
| [Caracteres ordenados](./ordered-characters) | Implementado, #21 |
| [Comparación de modelos](./model-comparison) por verosimilitudes marginales | Implementado en el CLI; concordancia con BEAST 2.7.8 registrada, #24 |
| [Datación de fósiles](./tip-dating) | Implementada en el motor y la app nativa con parámetros del proceso fijos o estimados, #25 / #40 / #41 |
| [Comprobaciones predictivas posteriores](./posterior-prediction) | Implementado en el CLI, #26 |
| Evaluación de repeticiones dentro de subárboles | Evaluado; experimental, desactivado por defecto, #17 |
| Adaptadores opcionales de JNI para BEAST 2 y de C ABI para BEAGLE | Posible, #6 y #7 |

## Establecer la línea base

El [núcleo](./kernel) se comprueba contra una implementación escalar independiente. El [lector de NEXUS](./nexus) suministra observaciones ponderadas sin BEAST. El [experimento de referencia](https://github.com/getcortexlabs/rove/blob/main/benchmarks/beast-2.7.8/2026-10-25/README.md) registra las entradas exactas, la configuración del modelo, dos ejecuciones completas de BEAST y estados numéricos fijos.

La [línea base en Apple M4 Max](https://github.com/getcortexlabs/rove/blob/main/benchmarks/rove/2026-10-26/README.md) registra 11 casos en Rust, separando preparación, evaluaciones completas, actualizaciones parciales y gestión del estado de las propuestas. Incluye muestras originales, observaciones de asignación de memoria y variación entre tres procesos. Una [comparación del modelo completo](https://github.com/getcortexlabs/rove/blob/main/benchmarks/inference/2026-10-29/README.md) ya registra tiempo y ESS por segundo en seis casos de 20 taxones; el [informe ampliado](https://github.com/getcortexlabs/rove/blob/main/benchmarks/expanded/2026-10-30/README.md) añade ejecuciones con 90 y 500 taxones y comparaciones de los núcleos BEAST/BEAGLE, con los límites de convergencia registrados. La comparación con reloj estricto, más abajo, registra el tiempo transcurrido de ambos motores en una configuración pequeña, pero no es un benchmark controlado.

## Corrección antes que optimización

Mkv condiciona la verosimilitud al proceso de observación para matrices con solo caracteres variables. Reproduce los dos estados archivados de BEAST, y sus patrones constantes se mantienen separados de los pesos de los patrones observados.

La ruta directa de Mk y el backend NEON están implementados y medidos en los informes de [Mk](https://github.com/getcortexlabs/rove/blob/main/benchmarks/rove/2026-10-28/README.md) y [NEON](https://github.com/getcortexlabs/rove/blob/main/benchmarks/neon/2026-10-28/README.md). El reescalado por potencias de dos elimina los logaritmos por nodo y guarda los totales de exponentes por subárbol. Su [comparación](https://github.com/getcortexlabs/rove/blob/main/benchmarks/scaling/2026-10-30/README.md) separa ambas mejoras en los dos backends, incluye ejecuciones del muestreador y registra los casos más lentos. La implementación portable sigue siendo la referencia y alternativa.

## Inferencia independiente

El primer [muestreador mínimo](./sampler) admitía Mk o Mkv binario, un reloj estricto y el prior de nacimiento-muerte. Con los mismos movimientos y pasos fijos que BEAST 2.7.8, sus medias posteriores y frecuencias de clados en la matriz de referencia cumplieron criterios fijados antes de las ejecuciones ([informe](https://github.com/getcortexlabs/rove/blob/main/benchmarks/strict-clock/2026-10-28/README.md)). El [modelo de referencia](./reference-model), con categorías gamma y reloj relajado lognormal, ya reproduce ambos estados archivados componente por componente. Los resúmenes de posterior y las frecuencias de clados cumplieron los criterios fijados antes de las ejecuciones en seis casos construidos con 20 taxones ([informe](https://github.com/getcortexlabs/rove/blob/main/benchmarks/inference/2026-10-29/README.md)). La validación avanza desde verosimilitudes en estados fijos hasta ejecuciones completas sobre matrices conocidas. La calidad del muestreo, incluidos el ESS por unidad de tiempo y los diagnósticos de árboles, cuenta tanto como el tiempo de ejecución.

La [comparación de modelos](./model-comparison) estima verosimilitudes marginales por stepping stone y path sampling (#24); el [informe](https://github.com/getcortexlabs/rove/blob/main/benchmarks/marginal-likelihood/2026-10-31/README.md) registra la concordancia con el PathSampler de BEAST 2.7.8 en dos configuraciones de reloj estricto. La [datación de fósiles](./tip-dating) permite fijar o estimar parámetros del proceso y origen en el motor, CLI y app nativa (#25 / #40 / #41). La estimación usa un modelo conjunto explícito de muestreo, validado con componentes de BEAST 2 SA y distribuciones independientes en árboles pequeños. Los ancestros muestreados siguen sin admitirse; estas densidades no son verosimilitudes marginales.

La [vista previa nativa para Mac](./macos-preview) importa y muestra matrices y prepara los ajustes del análisis. Ejecuta el motor Rust incluido con progreso real, cancelación y reapertura de configuraciones con verificación de huella. Los adaptadores de BEAST y BEAGLE son rutas de compatibilidad opcionales, no dependencias del motor en Rust.

Sigue los [issues del repositorio](https://github.com/getcortexlabs/rove/issues) para ver los criterios de aceptación y los resultados.

Las particiones por número de estados (#19) están implementadas en el motor, CLI, app nativa y checkpoints. Las reglas y la validación en estados fijos están en la [guía NEXUS](./nexus#particiones-por-numero-de-estados).

El condicionamiento a caracteres informativos (#20) está implementado en el motor, CLI, app nativa y checkpoints. Consulta [selección de caracteres](./character-selection) para las reglas de entrada y la validación.

La simulación predictiva posterior y las estadísticas morfológicas pareadas están disponibles mediante `rove predict`. Consulta [comprobaciones predictivas posteriores](./posterior-prediction) para conocer reglas de observación, interpretación y límites.
