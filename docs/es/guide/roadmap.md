---
description: "El orden de implementación de Rove: establecer la corrección y líneas base medidas, después optimizar el kernel y construir la inferencia independiente."
---

# Roadmap

Rove está en una etapa inicial. No hay versiones publicadas, ni crate publicado, ni fechas. Esto es el orden de implementación, no un calendario.

| Paso | Estado |
| --- | --- |
| Núcleo de verosimilitud escalar y pruebas numéricas de referencia | Hecho |
| CI en ARM64, x86_64 y la versión mínima de Rust | Hecho |
| Análisis de referencia reproducible con BEAST 2.7.8 | Hecho |
| NEXUS morfológico y compresión de patrones observados | Hecho |
| Línea base del núcleo Rust | Hecho, #8 |
| Benchmarks de inferencia completa y entre motores | Comparación con 20 taxones registrada; ampliación pendiente, #8 |
| Corrección de ascertainment de Mkv | Hecho |
| Poda directa de Mk | Hecho |
| MCMC independiente mínimo | Hecho |
| Vectorización NEON | Hecho |
| Modelo de referencia con reloj relajado | Hecho, #16 |
| Validación de inferencia completa | Hecho en seis casos de 20 taxones, #12 |
| Eficiencia del muestreo con reloj relajado | Siguiente, #27 |
| Vista previa nativa para importar y preparar análisis | Implementado, #28 |
| Ejecución y progreso en la aplicación | Implementado, #29 |
| Historial de ejecuciones y reutilización de ajustes | Implementado, #30 |
| Selección explícita del grupo externo y restricciones aplicadas | Implementado, #31 |
| Resumen MCC posterior y exportación anotada | Implementado, #32 |
| Cadenas en paralelo e informes de convergencia | Planificado, #23 |
| Reescalado por potencias de dos | Planificado, #18 |
| Checkpoints y reanudación de ejecuciones | Planificado, #22 |
| Particiones por número de estados | Planificado, #19 |
| Condicionamiento a caracteres informativos para parsimonia | Planificado, #20 |
| Caracteres ordenados | Planificado, #21 |
| Verosimilitudes marginales para comparar modelos | Planificado, #24 |
| Prior de nacimiento-muerte fosilizado con fechas de terminales | Planificado, #25 |
| Comprobaciones predictivas posteriores | Planificado, #26 |
| Evaluación de repeticiones dentro de subárboles | Planificado, #17 |
| Adaptadores opcionales de JNI para BEAST 2 y de C ABI para BEAGLE | Posible, #6 y #7 |

## Establecer la línea base

El [núcleo](./kernel) se comprueba contra una implementación escalar independiente. El [lector de NEXUS](./nexus) suministra observaciones ponderadas sin BEAST. El [experimento de referencia](https://github.com/getcortexlabs/rove/blob/main/benchmarks/beast-2.7.8/2026-10-25/README.md) registra las entradas exactas, la configuración del modelo, dos ejecuciones completas de BEAST y estados numéricos fijos.

La [línea base en Apple M4 Max](https://github.com/getcortexlabs/rove/blob/main/benchmarks/rove/2026-10-26/README.md) registra 11 casos en Rust, separando preparación, evaluaciones completas, actualizaciones parciales y gestión del estado de las propuestas. Incluye muestras originales, observaciones de asignación de memoria y variación entre tres procesos. Una [comparación del modelo completo](https://github.com/getcortexlabs/rove/blob/main/benchmarks/inference/2026-10-29/README.md) ya registra tiempo y ESS por segundo en seis casos de 20 taxones; las cargas mayores siguen pendientes. La comparación con reloj estricto, más abajo, registra el tiempo transcurrido de ambos motores en una configuración pequeña, pero no es un benchmark controlado.

## Corrección antes que optimización

Mkv condiciona la verosimilitud al proceso de observación para matrices con solo caracteres variables. Reproduce los dos estados archivados de BEAST, y sus patrones constantes se mantienen separados de los pesos de los patrones observados.

La ruta directa de Mk sustituye el producto genérico de matrices en los modelos Mk. En un Apple M4 Max, la ruta de matrices tardó 1,25 veces lo que la ruta Mk en una evaluación completa de la matriz de referencia, y 2,66 veces con ocho estados ([informe](https://github.com/getcortexlabs/rove/blob/main/benchmarks/rove/2026-10-28/README.md)). Después, el backend NEON fusionó la actualización de nodo de Mk en una sola pasada por bloques de ocho patrones. En la misma máquina, el código portable tardó 1,48 veces lo que NEON en una evaluación completa de la matriz de referencia, entre 1,10 y 1,32 veces en los casos 90 × 250 con 250 patrones y entre 1,08 y 1,11 veces en una ejecución del muestreador ([informe](https://github.com/getcortexlabs/rove/blob/main/benchmarks/neon/2026-10-28/README.md)). El logaritmo del reescalado, una llamada escalar por patrón y nodo, ocupa ahora cerca de dos quintos del tiempo de las actualizaciones de nodo en una ejecución del muestreador. La implementación portable sigue siendo la referencia y el respaldo. Cada optimización necesita una medición antes y después, no solo un recuento de operaciones aritméticas.

## Inferencia independiente

El [muestreador mínimo](./sampler) admite Mk o Mkv binario, un reloj estricto y el prior de nacimiento-muerte. Con los mismos movimientos y pasos fijos que BEAST 2.7.8, sus medias posteriores y frecuencias de clados en la matriz de referencia cumplieron criterios fijados antes de las ejecuciones ([informe](https://github.com/getcortexlabs/rove/blob/main/benchmarks/strict-clock/2026-10-28/README.md)). El [modelo de referencia](./reference-model), con categorías gamma y reloj relajado lognormal, ya reproduce ambos estados archivados componente por componente. Los resúmenes de posterior y las frecuencias de clados cumplieron los criterios fijados antes de las ejecuciones en seis casos construidos con 20 taxones ([informe](https://github.com/getcortexlabs/rove/blob/main/benchmarks/inference/2026-10-29/README.md)). La validación avanza desde verosimilitudes en estados fijos hasta ejecuciones completas sobre matrices conocidas. La calidad del muestreo, incluidos el ESS por unidad de tiempo y los diagnósticos de árboles, cuenta tanto como el tiempo de ejecución.

Después de la eficiencia del muestreo (#27), las funciones planificadas siguen lo que necesitan los análisis morfológicos: cadenas en paralelo con informes de convergencia (#23), checkpoints (#22), particiones por número de estados (#19), condicionamiento a caracteres informativos para parsimonia (#20), caracteres ordenados (#21), verosimilitudes marginales para comparar modelos (#24), el prior de nacimiento-muerte fosilizado con fechas de terminales (#25) y comprobaciones predictivas posteriores (#26). RevBayes documenta cada una de ellas para datos morfológicos y puede servir de segunda referencia donde BEAST no tenga una función equivalente.

La [vista previa nativa para Mac](./macos-preview) importa y muestra matrices y prepara los ajustes del análisis. Ejecuta el motor Rust incluido con progreso real, cancelación y reapertura de configuraciones con verificación de huella. Los adaptadores de BEAST y BEAGLE son rutas de compatibilidad opcionales, no dependencias del motor en Rust.

Sigue los [issues del repositorio](https://github.com/getcortexlabs/rove/issues) para ver los criterios de aceptación y los resultados.
