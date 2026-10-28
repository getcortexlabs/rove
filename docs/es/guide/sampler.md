---
description: "Cómo ejecutar el primer muestreador independiente de Rove sobre una matriz morfológica binaria: el modelo, el comando, los archivos que escribe, las propuestas y las comprobaciones que las respaldan."
---

# Muestreo de árboles

`rove sample` ejecuta un análisis de Monte Carlo por cadenas de Markov sobre una matriz morfológica binaria, sin BEAST. El modelo es pequeño a propósito. La variación de tasas gamma, el reloj relajado y los demás componentes del análisis de referencia están en el [roadmap](./roadmap).

## El modelo

- **Caracteres.** Una partición de caracteres binarios bajo Mk con frecuencias iguales y una categoría de tasa. Por defecto, la verosimilitud se condiciona a caracteres variables con la [corrección Mkv](./kernel#condicionar-a-caracteres-variables); `--conditioning none` usa Mk sin condicionar.
- **Reloj.** Estricto, con la tasa fija en 1. Por eso las alturas de los nodos están en cambios esperados por carácter, la convención del análisis de referencia, que no tiene calibraciones. Todos los terminales están a altura cero.
- **Prior del árbol.** El prior de nacimiento–muerte de Gernhard (2008), calculado como BEAST 2.7 calcula `BirthDeathGernhard08Model` con `type="unscaled"`: todos los linajes están muestreados y la densidad se condiciona al número de terminales, pero no a la altura de la raíz. Sus parámetros son r = λ − μ, con prior Uniform(0, 1000), y a = μ/λ, con prior Uniform(0, 1). Son las definiciones del XML de referencia, donde los parámetros se llaman `BDBirthRate` y `BDDeathRate`.
- **Restricciones.** Cada opción `--monophyly` enumera taxones que deben formar un clado. Las propuestas que violarían una restricción se rechazan. Las restricciones deben estar anidadas o ser disjuntas.

## Ejecutarlo

```sh
cargo build --release
target/release/rove sample fixtures/reference/20x50/matrix.nex \
  --output runs/seed-1 --seed 1 --steps 10000000 --sample-every 1000 \
  --monophyly T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11,T12,T13,T14,T15,T16,T17,T18,T19
```

| Opción | Significado |
| --- | --- |
| `--output <directorio>` | Directorio para los archivos de salida. Todavía no debe existir. |
| `--seed <n>` | Semilla del generador de números aleatorios. Opción obligatoria. |
| `--steps <n>` | Número de pasos de la cadena. Opción obligatoria. |
| `--sample-every <n>` | Pasos entre muestras; debe dividir a `--steps`. El estado inicial es la muestra 0. Opción obligatoria. |
| `--monophyly <taxones>` | Nombres de taxones, separados por comas, que deben formar un clado; como en la matriz, `_` equivale a un espacio. Se puede repetir. |
| `--gaps reject\|missing` | Rechazar las matrices con gaps (por defecto) o leer los gaps como datos faltantes. |
| `--conditioning variable\|none` | Mkv (por defecto) o Mk. |
| `--sample-from-prior` | Ignorar los caracteres y muestrear el prior. |
| `--no-tuning` | Mantener fijos todos los tamaños de desplazamiento y factores de escala. |
| `--tuning <movimiento>=<valor>` | Tamaño inicial de desplazamiento o factor de escala de un movimiento, como `subtree_slide=0.001`. Se puede repetir. |
| `--backend portable\|neon` | El [backend](./kernel#backends) de la verosimilitud: el código portable o el kernel NEON (por defecto en aarch64). Ambos dan las mismas muestras. |
| `--print-every <n>` | Pasos entre líneas de progreso en la salida de error estándar; por defecto, una vigésima parte de la cadena. |

El comando se detiene con un mensaje ante entradas que no admite: caracteres con más de dos estados, particiones o bloques de modelo en el archivo NEXUS, caracteres que podrían ser constantes bajo Mkv, nombres de taxones con caracteres distintos de letras, dígitos, espacios, `_`, `.` y `-` (o con espacios y `_` a la vez), taxones desconocidos en una restricción, restricciones solapadas que no están anidadas, valores de ajuste fuera de rango, un backend que la compilación no incluya y opciones que no conoce o que aparecen dos veces.

Con el mismo ejecutable, la misma entrada, la misma configuración y la misma semilla, en la misma plataforma, una ejecución produce las mismas muestras, sea cual sea el backend. Los logaritmos y las exponenciales vienen de la biblioteca matemática de la plataforma, así que otra plataforma puede diferir en los últimos dígitos y, a partir de ahí, divergir.

## Archivos de salida

- **`trace.log`** está separado por tabuladores, después de líneas que empiezan por `#` y registran la versión, el comando y el SHA-256 de la entrada y del ejecutable. Sus columnas son `Sample`, `posterior`, `likelihood`, `prior`, `birth_death` (la log-densidad del prior del árbol), `birth_difference_rate`, `relative_death_rate`, `tree_height` y `tree_length`. Tiene el formato de una traza de BEAST, y el LogAnalyser de BEAST la lee.
- **`trees.nexus`** es un bloque de árboles NEXUS con una tabla de traducción y un árbol por muestra, que el NexusParser de BEAST lee. Los espacios de los nombres de taxones se escriben como `_`. Las longitudes de rama están en cambios esperados por carácter.
- **`run.json`** registra los argumentos del comando, la plataforma, la ruta y el SHA-256 de la entrada, el SHA-256 del ejecutable tomado al empezar la ejecución, el modelo, toda la configuración (incluido el backend), el peso, el ajuste y los recuentos de aceptación de cada movimiento, la posterior final y el tiempo transcurrido.

## Propuestas

Cada movimiento sigue la definición y la razón de Hastings del operador de BEAST 2.7 de la segunda columna. Los pesos son los de la comparación con BEAST bajo reloj estricto.

| Movimiento | Operador de BEAST 2 | Qué cambia | Peso |
| --- | --- | --- | ---: |
| `node_height` | `Uniform` | La altura de un nodo interno que no es la raíz, uniforme entre su hijo más alto y su padre | 30 |
| `subtree_slide` | `SubtreeSlide` | Desplaza un nodo hacia arriba o hacia abajo de su linaje con un paso gaussiano, y reinjerta su subárbol cuando pasa por otro nodo | 15 |
| `narrow_exchange` | `Exchange`, narrow | Intercambia un nodo con su tío | 15 |
| `wide_exchange` | `Exchange`, wide | Intercambia dos subárboles cuando las alturas lo permiten | 3 |
| `wilson_balding` | `WilsonBalding` | Poda un subárbol y lo reinjerta en otra rama, nunca en la raíz | 3 |
| `root_scale` | `ScaleOperator`, solo la raíz | Escala la altura de la raíz | 3 |
| `tree_scale` | `ScaleOperator` sobre el árbol | Escala todas las alturas internas | 3 |
| `birth_difference_scale` | `ScaleOperator` | Escala λ − μ | 3 |
| `relative_death_scale` | `ScaleOperator` | Escala μ/λ | 3 |

El tamaño del desplazamiento y los factores de escala se ajustan durante la ejecución hacia una tasa de aceptación de 0,234, con la regla de actualización de BEAST. El ajuste empieza después de 10.000 propuestas ajustables. Después de la `n`-ésima propuesta de un movimiento desde entonces, el paso es `(α − 0,234)/√(n + 1)`: es la transformación opcional de raíz cuadrada de BEAST, mientras que la opción por defecto de BEAST divide por `n + 1`. Una propuesta no válida, o que viola una restricción, cuenta como fallida y entra en el ajuste con `α = 0`. Para una propuesta no válida, BEAST reutiliza en cambio el α del paso anterior. Con `--no-tuning` los valores quedan fijos, lo que hace comparables las propuestas de dos motores.

## Estado y restauración

Antes de cada paso, el muestreador guarda el árbol y los parámetros de tasa, y registra los índices de los búferes del [núcleo de verosimilitud](./kernel#nodos-y-buferes). Después de una propuesta, compara el árbol nuevo con el guardado. Cada rama cuya longitud o cuyo padre cambió recibe la distancia nueva en un búfer nuevo, y solo se recalculan los parciales de sus ancestros. Un paso rechazado recupera el árbol guardado, los parámetros y los búferes del núcleo. Los movimientos de λ − μ y μ/λ no tocan la verosimilitud.

## Comprobarlo

`tests/sampler.rs` y las pruebas unitarias de `src/mcmc/moves.rs` comprueban el muestreador frente a resultados conocidos de antemano:

- **El prior, con cinco terminales.** Bajo este prior, todas las topologías ordenadas son igual de probables, así que la frecuencia de cada topología es proporcional al número de formas de ordenar sus nodos internos. Si F es la función de distribución de los tiempos de especiación, F(altura de la raíz) sigue una Beta(4, 2) y la media de F sobre los nodos internos es 5/12. Se comprueba con los movimientos de altura solos, con todos los movimientos de árbol juntos y con una restricción de monofilia.
- **Cada movimiento de topología por separado.** Bajo el prior de nacimiento–muerte, bajar un nodo siempre aumenta la densidad, así que todo desplazamiento hacia abajo se acepta sea cual sea su término de Hastings. Un objetivo sintético cuya densidad crece con la altura de los nodos, en el que cada topología sigue ponderada por sus ordenaciones, comprueba la razón de Hastings de cada movimiento. Quitar los recuentos de la razón del *subtree slide* multiplica por tres, aproximadamente, su distancia a las frecuencias exactas.
- **El movimiento de escala** mantiene un objetivo uniforme en [0, 1], y λ − μ y μ/λ mantienen sus priors uniformes cuando la cadena muestrea el prior con todos los movimientos.
- **El ajuste** mueve cada valor en la dirección correcta y en la medida correcta, comprobado frente a un paso calculado a mano.
- **Gestión del estado.** En una matriz de seis taxones con celdas faltantes e inciertas, después de cada paso de cadenas de 4.000 pasos bajo Mkv y Mk, la posterior guardada coincide con una evaluación desde cero, un paso rechazado restaura exactamente el estado anterior, la restricción se mantiene y cada movimiento se acepta y se rechaza al menos una vez.
- **El prior del árbol** reproduce el valor de BEAST 2.7.8 en los dos estados archivados del análisis de referencia, salvo una diferencia menor que 10⁻¹².
- **Estados fijos.** En el estado final de cada cadena de BEAST de la comparación siguiente, la verosimilitud y la posterior de Rove reproducen las de BEAST 2.7.8 con una diferencia menor que 10⁻¹⁰, y el prior del árbol y el prior, con una diferencia menor que 10⁻¹².
- **Backends.** La misma cadena con todos los backends que incluye la compilación hace los mismos movimientos y llega a la misma posterior, bit a bit, en cada paso.

`tests/allocations.rs` comprueba que 20.000 pasos sobre la matriz de referencia no hacen ninguna asignación de memoria en el heap. `tests/cli.rs` ejecuta el comando, comprueba los tres archivos y las configuraciones rechazadas, y comprueba que cerrar la salida de error estándar no detiene una ejecución.

## Comparación con BEAST

Rove y BEAST 2.7.8 muestrearon la matriz de referencia con este modelo, con y sin la corrección Mkv, usando los mismos movimientos, pesos y tamaños de paso fijos, con cuatro cadenas cada uno. Cada media posterior quedó a no más de cuatro errores estándar de Monte Carlo combinados de la de BEAST, y cada frecuencia de clado a no más de 0,05: los criterios fijados antes de las ejecuciones. Las tasas de aceptación no formaban parte de los criterios; con 36 cadenas Mkv por motor, ningún movimiento difirió en más de 1,7 errores estándar.

La matriz no tiene homoplasia, así que con Mkv la verosimilitud no penaliza los árboles cortos: la altura posterior del árbol es de unos 0,003 cambios esperados por carácter, la posterior de λ − μ queda cortada por el límite superior de su prior, 1000, y la topología apenas se mueve. Con Mk, la topología de referencia tiene una frecuencia posterior de entre 0,82 y 0,83 en todas las cadenas de ambos motores. El [informe](https://github.com/getcortexlabs/rove/blob/main/benchmarks/strict-clock/2026-10-28/README.md) recoge las cifras, los tiempos de ejecución y los comandos.

## Limitaciones

- Una partición de caracteres binarios. La variación de tasas gamma, los relojes relajados, las fechas de los terminales, las calibraciones y otros priors de árbol no están implementados; los componentes del análisis de referencia se siguen en [#16](https://github.com/getcortexlabs/rove/issues/16).
- El árbol inicial es aleatorio, con tiempos de nodo parecidos a los de un coalescente, y cumple las restricciones.
- Una cadena en un hilo. Una ejecución no se puede reanudar desde un checkpoint.
- El comando no calcula diagnósticos de convergencia. Los scripts del repositorio en `tools/beast` los calculan a partir de su salida.
