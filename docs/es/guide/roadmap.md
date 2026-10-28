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
| Benchmarks de inferencia completa y entre motores | Pendiente, #8 |
| Corrección de ascertainment de Mkv | Hecho |
| Poda directa de Mk | Siguiente, #4 |
| MCMC independiente mínimo | Planificado, #15 |
| Vectorización NEON | Planificado, #5 |
| Modelo de referencia con reloj relajado | Planificado, #16 |
| Validación de la inferencia completa | Planificado, #12 |
| Evaluación de repeticiones dentro de subárboles | Planificado, #17 |
| Adaptadores opcionales de JNI para BEAST 2 y de C ABI para BEAGLE | Posible, #6 y #7 |

## Establecer la línea base

El [núcleo](./kernel) se comprueba contra una implementación escalar independiente. El [lector de NEXUS](./nexus) suministra observaciones ponderadas sin BEAST. El [experimento de referencia](https://github.com/getcortexlabs/rove/blob/main/benchmarks/beast-2.7.8/2026-10-25/README.md) registra las entradas exactas, la configuración del modelo, dos ejecuciones completas de BEAST y estados numéricos fijos.

La [línea base en Apple M4 Max](https://github.com/getcortexlabs/rove/blob/main/benchmarks/rove/2026-10-26/README.md) registra 11 casos en Rust, separando preparación, evaluaciones completas, actualizaciones parciales y gestión del estado de las propuestas. Incluye muestras originales, observaciones de asignación de memoria y variación entre tres procesos. Las comparaciones de inferencia completa y entre motores siguen pendientes; no se ha medido ninguna aceleración entre motores.

## Corrección antes que optimización

Mkv condiciona la verosimilitud al proceso de observación para matrices con solo caracteres variables. Reproduce los dos estados archivados de BEAST, y sus patrones constantes se mantienen separados de los pesos de los patrones observados.

La ruta directa de Mk sustituye el producto genérico de matrices en los modelos Mk. NEON en Apple silicon llega después del muestreador mínimo, para que los perfiles de ejecuciones completas muestren dónde pesa el tiempo del kernel. La implementación escalar sigue siendo la referencia y el respaldo. Cada optimización necesita una medición antes y después, no solo un recuento de operaciones aritméticas.

## Inferencia independiente

Un muestreador mínimo viene antes de la configuración completa con reloj relajado y nacimiento-muerte. La validación avanza desde verosimilitudes en estados fijos hasta ejecuciones completas sobre matrices conocidas. La calidad del muestreo, incluidos el ESS por unidad de tiempo y los diagnósticos de árboles, cuenta tanto como el tiempo de ejecución.

El producto previsto es una aplicación nativa para macOS. El trabajo en la interfaz de la aplicación no ha comenzado. Los adaptadores de BEAST y BEAGLE son rutas de compatibilidad opcionales, no dependencias del motor en Rust.

Sigue los [issues del repositorio](https://github.com/getcortexlabs/rove/issues) para ver los criterios de aceptación y los resultados.
