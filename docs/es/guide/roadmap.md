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
| Línea base de benchmarks en Rust | Siguiente, #8 |
| Corrección de ascertainment de Mkv | Planificado, #14 |
| Poda directa de Mk | Planificado, #4 |
| Vectorización NEON | Planificado, #5 |
| MCMC independiente mínimo | Planificado, #15 |
| Modelo de referencia con reloj relajado | Planificado, #16 |
| Validación de la inferencia completa | Planificado, #12 |
| Evaluación de repeticiones dentro de subárboles | Planificado, #17 |
| Adaptadores opcionales de JNI para BEAST 2 y de C ABI para BEAGLE | Posible, #6 y #7 |

## Establecer la línea base

El [núcleo](./kernel) se comprueba contra una implementación escalar independiente. El [lector de NEXUS](./nexus) suministra observaciones ponderadas sin BEAST. El [experimento de referencia](https://github.com/getcortexlabs/rove/blob/main/benchmarks/beast-2.7.8/2026-10-25/README.md) registra las entradas exactas, la configuración del modelo, dos ejecuciones completas de BEAST y estados numéricos fijos.

El siguiente paso es medir la implementación actual en Rust antes de cambiar sus rutas numéricas. Los benchmarks deben distinguir la preparación de los cálculos repetidos de verosimilitud y registrar entradas, versiones, hardware, opciones de compilación y comandos. No se ha medido ninguna aceleración entre motores.

## Corrección antes que optimización

Mkv condiciona la verosimilitud al proceso de observación para matrices con solo caracteres variables. Necesita sus propias pruebas contra los estados de referencia fijos; los patrones artificiales de ascertainment deben mantenerse separados de los pesos de los patrones observados.

Después, la ruta directa de Mk puede reemplazar la multiplicación genérica de matrices, seguida de NEON en Apple silicon. La implementación escalar sigue siendo la referencia y el respaldo. Cada optimización necesita una medición antes y después, no solo un recuento de operaciones aritméticas.

## Inferencia independiente

Un muestreador mínimo viene antes de la configuración completa con reloj relajado y nacimiento-muerte. La validación avanza desde verosimilitudes en estados fijos hasta ejecuciones completas sobre matrices conocidas. La calidad del muestreo, incluidos el ESS por unidad de tiempo y los diagnósticos de árboles, cuenta tanto como el tiempo de ejecución.

El producto previsto es una aplicación nativa para macOS. El trabajo en la interfaz de la aplicación no ha comenzado. Los adaptadores de BEAST y BEAGLE son rutas de compatibilidad opcionales, no dependencias del motor en Rust.

Sigue los [issues del repositorio](https://github.com/getcortexlabs/rove/issues) para ver los criterios de aceptación y los resultados.
