---
description: "Simula matrices morfológicas desde una ejecución completada de Rove y compara discrepancias predictivas posteriores pareadas."
---

# Comprobaciones predictivas posteriores

`rove predict` comprueba si el modelo ajustado puede generar matrices con propiedades similares a las observadas. Usa árboles y parámetros de la posterior, genera caracteres y compara la matriz original y la simulada en el **mismo árbol muestreado**. Evalúa aspectos de la adecuación del modelo; no establece convergencia ni la probabilidad de que el modelo sea verdadero. El [tutorial de morfología de RevBayes](https://revbayes.github.io/tutorials/pps_morpho/) presenta el enfoque.

La función está disponible mediante el CLI en Rust. Todavía no hay una pantalla nueva en la aplicación para Mac. No requiere BEAST, Java, R ni RevBayes.

## Ejecutar una comprobación

Primero revisa los diagnósticos de muestreo y convergencia. Proporciona un directorio completado con `run.json`, `trace.log` y `trees.nexus`, junto con su entrada NEXUS original exacta:

```sh
cargo run --release --bin rove -- predict runs/seed-1 \
  --matrix fixtures/reference/20x50/matrix.nex \
  --output runs/prediction-1 --burnin-percent 25 \
  --replicates 100 --seed 42
```

Usa la matriz realmente analizada, incluida cualquier selección de taxones realizada en la aplicación. Su SHA-256 debe coincidir con el registro. En análisis paralelos, selecciona un directorio de una cadena fría completada; este comando no combina cadenas. Rechaza directorios de salida existentes, ejecuciones solo del prior, archivos incompletos y estados incompatibles entre trazas y árboles. Los archivos fuente se copian al nuevo directorio y permanecen intactos.

El burn-in descarta `floor(total de muestras × porcentaje / 100)` filas, contando `STATE_0` en el total. Cada réplica elige uniformemente una fila retenida **con reemplazo** y recibe una semilla registrada. Es normal repetir filas. Antes de simular, reconstruye la verosimilitud ajustada de cada fila elegida para comprobar la combinación de topología, tasas de ramas y forma gamma.

## Reglas del modelo y las observaciones

- Mantener el alfabeto y orden ajustados de cada carácter, incluso si faltan estados posibles en una réplica. Los caracteres ordenados conservan estados intermedios. No volver a inferir alfabetos a partir de las observaciones simuladas.
- Usar el reloj estricto ajustado o las tasas explícitas del reloj relajado, con la mezcla gamma ajustada de cuatro categorías cuando exista.
- Para Mkv y Mkparsinf, rechazar los **caracteres latentes completos** excluidos y después restaurar la máscara original de datos ausentes y gaps. Volver a sortear la categoría gamma en cada intento. Una réplica enmascarada puede parecer constante aunque su carácter latente sea variable; es intencional.
- Admitir estados únicos, datos ausentes y gaps tratados explícitamente como ausentes. Rechazar conjuntos de incertidumbre y polimorfismo, que necesitan un modelo generador de observaciones. Mkparsinf ordenado sigue sin soporte, como en inferencia.

Los eventos de condicionamiento raros pueden superar el límite predeterminado de 100.000 intentos por carácter. El comando falla sin publicar resultados parciales; `--max-attempts` puede elevarlo hasta 10.000.000. La cancelación mediante el proceso propietario (`--cancel-on-stdin-close`) también elimina la salida parcial. Terminar el proceso a la fuerza puede dejar un directorio incompleto; solo un `predictive.json` completo indica que la comprobación terminó. Hay límites de réplicas, tamaño de salida y memoria de trabajo.

## Leer el resultado

`predictive.json` registra modelo, huellas de archivos y compilación, burn-in, semillas, alfabetos fijados y estimaciones de colas globales y por carácter. `replicates.jsonl` vincula cada matriz con su estado y semilla, con estadísticas pareadas. `characters.tsv` contiene puntuaciones pareadas por carácter. `matrices/` contiene réplicas NEXUS legibles; interpreta sus alfabetos usando `frozen_character_sets` del informe.

La parsimonia usa puntuaciones de Sankoff en cada árbol posterior, sin buscar otra topología óptima. Los cambios no ordenados cuestan uno; los ordenados, `|i − j|`. Para cada carácter, `s` es la puntuación en el árbol, `m` el mínimo posible y `g` el máximo posible. La consistencia global es `Σm / Σs`; la retención es `(Σg − Σs) / (Σg − Σm)`. Los denominadores cero son indefinidos, no cero. El número de caracteres informativos usa `g > m`, considerando los costes ordenados. Las observaciones ausentes no tienen coste de cambio.

El ajuste por carácter incluye el **logaritmo negativo de la verosimilitud no condicionada del patrón** en el estado posterior pareado, con la máscara original. Es una estadística de discrepancia, no una densidad predictiva condicionada. La selección de caracteres se aplica durante la simulación.

`p_upper` es la fracción de pares válidos cuya estadística simulada es mayor o igual a la observada; `p_lower` usa menor o igual. Los empates cuentan en ambas colas y se registran por separado. Los pares con CI/RI indefinidos se cuentan y excluyen. Las fracciones pequeñas indican discrepancias que investigar; no hay umbral automático de aprobación. Las comprobaciones por carácter son exploratorias, sin ajuste por pruebas múltiples.

Los intervalos de Wilson del 95% describen el error de simulación **condicionado a la posterior empírica finita de origen**. No incluyen error de MCMC. Aumentar las réplicas no corrige un muestreo posterior deficiente, y un resultado no extremo no demuestra adecuación.

## Validación

Los tests comparan frecuencias simuladas con probabilidades independientes en forma cerrada en árboles de cuatro terminales, incluidas mezclas gamma condicionadas y máscaras de ausencias. La enumeración exhaustiva de estados ancestrales y topologías de cuartetos comprueba parsimonia, consistencia, retención y verosimilitudes para caracteres ordenados y no ordenados. Los tests del CLI cubren ambos relojes, todos los condicionamientos admitidos, reproducción, archivos incompatibles, cancelación y rechazo de resultados parciales.
