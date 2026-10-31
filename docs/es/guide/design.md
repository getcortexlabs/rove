---
description: "Cómo dispone Rove los datos de verosimilitud, por qué el kernel tiene en cuenta el modelo, para qué sirve la referencia escalar y cómo funcionan los búferes y el reescalado."
---

# Diseño

Rove es un único crate de Rust. Calcula la verosimilitud de un árbol binario enraizado bajo el modelo Mk para una matriz de caracteres discretos. Las decisiones siguientes son las que ya se ven en el código. El trabajo de SIMD que preparan empezó con el [backend NEON](./kernel#backends); el resto está en el [roadmap](./roadmap).

## Disposición por patrones

Un kernel de verosimilitud pasa su tiempo multiplicando matrices de transición por verosimilitudes parciales. Los kernels de CPU de BEAGLE vectorizan a lo largo de los estados y están especializados para cuatro estados, lo que conviene a los nucleótidos. Las matrices morfológicas son en su mayoría caracteres binarios y ternarios, así que esos kernels toman la ruta genérica con relleno.

Rove almacena los parciales al revés. Para cada categoría de tasa y cada estado hay una secuencia contigua de valores, uno por patrón de sitio, rellenada hasta un múltiplo de ocho lanes. El bucle interno recorre los patrones, así que tiene la misma forma para dos estados que para sesenta y uno.

`Layout` describe esa organización: el número de estados, categorías de tasa y patrones, y el stride con relleno. `offset(category, state)` da el inicio de una secuencia.

## Rutas rápidas específicas del modelo

Mk tiene una forma cerrada. La matriz de transición de una rama es `P(t) = e·I + (1 − e)·π`, donde `e` depende solo de la longitud de rama y `π` contiene las frecuencias de estados. Multiplicarla por un vector cuesta O(k) por patrón en lugar de O(k²).

Un núcleo Mk, construido con `LikelihoodCore::mk`, usa esa forma directamente. Cada rama guarda una distancia, `e` y `1 − e` por categoría de tasa en lugar de una matriz, y la actualización de un hijo calcula, para cada patrón, una sola suma ponderada por las frecuencias que sirve a todos los estados. El núcleo de matrices se mantiene para modelos sin una forma así, y las pruebas comparan ambos con la referencia.

En un Apple M4 Max, la [comparación de la ruta directa de Mk](https://github.com/getcortexlabs/rove/blob/main/benchmarks/rove/2026-10-28/README.md) midió los dos núcleos en los mismos procesos. En una evaluación completa de la referencia 20 × 50, el núcleo de matrices tardó 1,25 veces lo que el núcleo Mk; en una matriz binaria de 90 × 250, 1,11 veces; con ocho estados, 2,66 veces. Con dos estados, ambas rutas hacen casi las mismas operaciones, y la ruta Mk recorre la memoria menos veces.

## Una referencia escalar, siempre

`reference::Problem` es una implementación recursiva simple del algoritmo de poda de Felsenstein. Es lenta y simple a propósito. Las pruebas de igualdad construyen árboles aleatorios y comparan `LikelihoodCore` con ella, con estados en los terminales y con parciales en los terminales, con y sin reescalado, a una tolerancia relativa de 1e-9. `variable_log_likelihood` aplica la corrección Mkv de la misma forma directa.

Cada ruta optimizada que se añada después tiene que pasar las mismas pruebas. La referencia se queda.

## Búferes dobles y reescalado

Cada nodo posee dos búferes para parciales y dos para su rama: matrices de transición o distancias de Mk. Antes de que una propuesta sobrescriba un nodo, `set_node_partials_for_update` o `set_node_matrix_for_update` cambia ese nodo a su otro búfer, de modo que los valores anteriores sobreviven. `store` registra qué búferes son los actuales. `restore` recupera ese conjunto intercambiando, `unstore` copiando. Aceptar un movimiento no cuesta nada. Esto es lo que un muestreador MCMC necesita para rechazar un movimiento a bajo costo.

Con `Scaling::Always`, cada nodo reescala por una potencia de dos elegida a partir del máximo entre los estados y categorías de cada patrón. Su búfer guarda un exponente entero para todo el subárbol: el exponente local más los totales de ambos hijos. La raíz recupera la escala con `E * ln(2)` una vez por patrón, sin recorrer todos los nodos. Los exponentes se restauran junto con los búferes de parciales. Véase la [convención exacta](./kernel#reescalado).

## Patrones constantes en el mismo núcleo

La corrección Mkv necesita la probabilidad de cada patrón constante con el mismo árbol, las mismas tasas y el mismo escalado que las observaciones. Rove añade esos patrones a las entradas de los terminales del núcleo en lugar de calcularlos aparte, de modo que las actualizaciones parciales, el reescalado y el guardado/restauración los cubren sin cambios. En la referencia 20 × 50, los dos patrones adicionales elevan el stride con relleno de 24 a 32, y cada actualización de nodo recorre un tercio más de lanes.

## Un muestreador sobre los mismos búferes

El [muestreador](./sampler) mantiene el árbol y sus parámetros fuera del núcleo. Antes de cada paso, los guarda y registra los índices de los búferes del núcleo. Después de una propuesta, compara el árbol nuevo con el guardado, da a cada rama modificada una distancia nueva en su otro búfer y recalcula solo los ancestros de esas ramas. Un paso rechazado vuelve a intercambiar los índices. Las propuestas reproducen los operadores y las razones de Hastings de BEAST 2, así que una comparación con BEAST puede usar los mismos movimientos y pesos.

La configuración de referencia añade forma gamma, sigma del reloj y tasas explícitas de ramas al estado guardado. Cambiar solo sigma no altera la verosimilitud; el movimiento conjunto de sigma y tasas actualiza las distancias de las ramas; cambiar la forma actualiza las distancias por categoría en todo el árbol. Sus propuestas difieren de ORC, como recoge la [guía del modelo de referencia](./reference-model#propuestas-y-salida).

## Dónde emplea el tiempo el muestreador

El [perfil con 90 y 500 taxones](https://github.com/getcortexlabs/rove/blob/main/benchmarks/profile/2026-10-31/README.md) mide el muestreador actual en un Apple M4 Max. Una compilación opcional para benchmarks muestrea las fases según el tiempo transcurrido; no es un perfil nativo de las pilas de CPU. Ocho pares de ejecuciones reprodujeron los valores de la traza, los árboles y las estadísticas de los operadores de la compilación normal. El informe conserva las razones de tiempo de la instrumentación, la variación del entorno y ambas repeticiones. La compilación predeterminada no contiene código de perfilado.

Con el preset de referencia, los parciales de los nodos y el reescalado suman aproximadamente el 68% de las observaciones con 90 taxones y el 82% con 500. El reescalado por sí solo representa alrededor del 22–23%, incluso tras sustituir los logaritmos por exponentes binarios: todavía escala los parciales y acumula los exponentes de los hijos. Con estas fracciones instrumentadas, eliminar todo el costo del núcleo limitaría la mejora en la ventana observada a unas 3,14× y 5,50×, respectivamente, manteniendo la reducción en la raíz, los recorridos, las propuestas y la salida. Son límites condicionales, no promesas de aceleración en producción. El [estudio con cadenas independientes](https://github.com/getcortexlabs/rove/blob/main/benchmarks/convergence/2026-10-31/README.md) evalúa por separado la calidad del muestreo.

## Primero NEON, después AVX2

La disposición existe para que los bucles internos puedan vectorizarse a lo largo de los patrones. En las compilaciones de release, el compilador ya vectoriza los bucles portables, uno a uno: una actualización de nodo pone a cero dos acumuladores, les suma la contribución de cada estado, los combina con los parciales de los hijos, y el reescalado vuelve a leer los resultados para encontrar el máximo de cada patrón. El [backend NEON](./kernel#backends) hace en cambio la actualización de nodo de Mk en una sola pasada por bloques de ocho patrones, con las sumas y los máximos en registros. Para cada patrón realiza las mismas operaciones en el mismo orden, así que reproduce bit a bit los resultados del código portable, salvo el payload de los NaN, y las pruebas lo exigen. En un Apple M4 Max, el código portable tardó 1,48 veces lo que NEON en una evaluación completa de la matriz de referencia ([informe](https://github.com/getcortexlabs/rove/blob/main/benchmarks/neon/2026-10-28/README.md)). El reescalado ahora extrae exponentes binarios con operaciones enteras; NEON procesa dos patrones por registro. No hay ruta AVX2.

## Un solo crate

Rove no depende de BEAST ni de BEAGLE. BEAST es una referencia de validación y benchmarking. Una C ABI para BEAGLE y un adaptador de `LikelihoodCore` de BEAST 2 sobre JNI son posibles rutas de compatibilidad. El núcleo en Rust no requiere ninguno de los dos, y ninguno se está construyendo ahora.

## Fuera de alcance

Competir con el backend CUDA de BEAGLE en alineamientos grandes de nucleótidos. Para eso, usa la GPU.
