---
description: "Declara caracteres morfológicos ordenados, conserva estados intermedios y combina particiones ordenadas y no ordenadas."
---

# Caracteres ordenados

Un carácter ordenado cambia entre estados vecinos: `0 ↔ 1 ↔ 2`. Una rama puede empezar en `0` y terminar en `2`, pasando por `1`. El orden restringe cada cambio individual, no el cambio total de la rama. Mk no ordenado permite un cambio directo entre cualquier par de estados.

Rove lee esta elección del NEXUS. Añade un bloque `ASSUMPTIONS` **después** del bloque `DATA` o `CHARACTERS`:

```nexus
BEGIN ASSUMPTIONS;
  OPTIONS DEFTYPE=UNORD;
  TYPESET * character_order = ORD: 2-3 5 8;
END;
```

Los caracteres 2, 3, 5 y 8 son ordenados; los demás no. Los índices empiezan en uno. La aplicación muestra **Ordered** o **Unordered** junto a cada partición en la matriz y la revisión. Todavía no hay un editor de ordenación separado.

## Sintaxis y alfabetos

- `DEFTYPE` acepta `ORD` o `UNORD`; el valor predeterminado es `UNORD`. Se aplica a los caracteres omitidos del conjunto de tipos.
- Se admite un `TYPESET` con nombre y `*` opcional. El único conjunto se aplica con o sin asterisco. La forma estándar acepta índices, rangos, `.` para el último carácter, `ALL` y pasos como `1-7\2`.
- `(STANDARD)` es opcional. `(VECTOR)` exige un token `ORD` o `UNORD` por carácter, por ejemplo `TYPESET order (VECTOR) = ORD UNORD ORD;` para tres caracteres.
- Se rechazan múltiples conjuntos de tipos, solapamientos, conjuntos de caracteres con nombre, otros tipos de transformación, pesos y exclusiones. No se descartan supuestos sin avisar. Consulta [Lectura de NEXUS](./nexus) para el resto del formato.

El orden declarado en `SYMBOLS` define los vecinos; las etiquetas descriptivas no. Con `--partitioning observed`, los caracteres ordenados conservan el alfabeto desde el primer símbolo hasta el mayor estado observado o explícitamente posible. Con `SYMBOLS="0123"`, las observaciones `0`, `2`, `3` necesitan **cuatro** estados ordenados, incluido el intermedio `1` sin observar. Las mismas observaciones no ordenadas forman un modelo de tres estados.

Los datos ausentes no definen estados posibles. Un alfabeto binario declarado conserva ambos estados; en otros casos, se rechaza un alfabeto inferido con menos de dos estados. Si estados superiores no observados son biológicamente posibles, usa `--partitioning declared` para mantener el alfabeto completo. La API Rust `compress_sets` también acepta alfabetos explícitos; los ordenados deben ser contiguos y seguir el orden declarado.

Las particiones agrupan por número de estados y ordenación. La compresión nunca mezcla caracteres ordenados y no ordenados, aunque coincidan sus observaciones. Comparten árbol, reloj y forma gamma. `rove inspect` y `run.json` registran ordenación, índices originales y correspondencias de estados de cada partición.

## Modelo y límites

Para `k` estados, las tasas instantáneas entre vecinos son `k / (2(k − 1))`; las demás tasas fuera de la diagonal son cero, y cada diagonal es la suma negativa de sus tasas de salida. Las frecuencias estacionarias son uniformes. Así se espera un cambio por carácter y unidad de distancia de rama; con dos estados coincide con Mk binario uniforme.

El motor Rust calcula matrices de transición mediante uniformización positiva con escalado y cuadrados sucesivos, reutilizando buffers. Las particiones ordenadas usan la poda matricial general; todavía no hay un kernel NEON específico. Las particiones no ordenadas conservan la ruta directa Mk.

Se admiten verosimilitudes sin selección de caracteres y condicionamiento Mkv para caracteres variables, con relojes estrictos o relajados y categorías gamma. **Mkparsinf, para caracteres solo informativos, no está disponible para los ordenados** y se rechaza antes de ejecutar. El condicionamiento debe corresponder a la recopilación de la matriz; cambiarlo para evitar un error cambia el modelo estadístico. Consulta [selección de caracteres](./character-selection).

Los checkpoints conservan las matrices ordenadas y distancias en caché. Reanudar exige el mismo ejecutable, entrada y configuración; este cambio no migra checkpoints de versiones anteriores.

## Validación

El [caso de validación](https://github.com/getcortexlabs/rove/tree/main/fixtures/validation/ordered) registra 36 exponenciales matriciales independientes calculadas con 160 dígitos, una comprobación analítica de tres estados y 12 comparaciones de estados fijos con `Ordinal` de BEAST 2.7.8 / MM 1.2.1. La diferencia absoluta máxima de log-verosimilitud fue `1,82e-9`, entre Mk/Mkv, ambos relojes y tres escalas de árbol. Las pruebas también verifican matrices estocásticas, estados intermedios, rechazo de propuestas, reanudación y asignaciones de memoria tras la preparación.

Esto demuestra concordancia numérica en los estados probados, no convergencia posterior ni mejoras de velocidad para análisis ordenados. BEAST solo genera la referencia de validación; Rove no necesita que esté instalado.
