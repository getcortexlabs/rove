---
description: "El modelo independiente del XML suministrado: categorías gamma, tasas lognormales explícitas por rama, priors y validación numérica."
---

# El modelo de referencia

`rove sample --model reference` ejecuta el modelo probabilístico del XML
20 × 50 suministrado, sin BEAST. Usa el mismo Mk binario, la corrección Mkv
por defecto, el prior de nacimiento–muerte y las convenciones de parámetros.
La opción predeterminada `--model strict` conserva la configuración anterior,
con una categoría y reloj estricto. Ambas aceptan `--conditioning none` para
Mk sin corrección.

## Tasas y priors

| Componente | Configuración de referencia |
| --- | --- |
| Tasas de caracteres | Cuatro cuantiles centrales gamma, cada uno con peso 1/4, normalizados a media uno |
| Forma gamma | Estimada; Exponential(media=1), valor inicial 1; límites [0,001; 1000] |
| Tasas de ramas | Una tasa explícita por rama no raíz; LogNormal(μ=−σ²/2, σ) independientes, valor inicial 0,5; tasas positivas |
| Desviación estándar σ del reloj | Desviación estándar de los logaritmos de tasas; Gamma(forma=5, **escala**=0,2), valor inicial 0,2 |
| Multiplicadores de mutación y reloj | Ambos fijos en 1 |
| Parámetros de nacimiento–muerte | λ−μ ~ Uniform(0,1000), μ/λ ~ Uniform(0,1), valores iniciales 1 y 0,5 |
| Prior del árbol | Gernhard, `unscaled`, todas las líneas muestreadas, condicionado al número de terminales, no a la altura de la raíz |
| Monofilia | Indicada mediante `--monophyly`; el comando de referencia restringe T1–T19 |

Los cuantiles gamma son 1/8, 3/8, 5/8 y 7/8, no las medias de cuatro
intervalos de probabilidad. Los límites de forma también se aplican al
muestrear el prior; como en BEAST, su densidad registrada no añade una
constante de normalización por truncamiento. Sigma es la desviación estándar
de los **logaritmos de las tasas**, no la media ni la desviación de las tasas
en espacio real. La media de la distribución lognormal es uno; las tasas
realizadas no se normalizan a media uno.

El vector de tasas tiene `2 × taxones − 2` entradas. Un nodo no raíz usa su
índice; si el último nodo no es la raíz, usa el índice de la raíz. Es el
mapeo de tasas explícitas de BEAST, incluso cuando un movimiento de subárbol
cambia la raíz. La raíz no tiene tasa de rama. La distancia genética en
categoría `c` es `diferencia de alturas × tasa de rama × tasa de categoría[c]`.

Al iniciar el reloj, BEAST reemplaza el límite inferior 1e-100 del XML
por cero. Rove acepta toda tasa positiva y finita; cero tiene densidad
lognormal nula.

## Ejecutar la matriz suministrada

Desde la raíz del repositorio:

```sh
cargo build --release --locked
sh fixtures/reference/20x50/rove.sh runs/reference-1 1
```

El script ejecuta diez millones de pasos, muestrea cada mil y aplica la
restricción T1–T19. El directorio de salida no debe existir. Equivale al
comando de [Muestreo de árboles](./sampler), añadiendo `--model reference`.
El soporte uno de la restricción se impone, no se recupera de los datos.

## Propuestas y salida

Los nueve movimientos clásicos de la guía conservan sus pesos. Se añaden
tres escalas: `gamma_shape_scale` (peso 0,1), `clock_sigma_scale` (3) y
`branch_rate_scale` (20), que elige una tasa uniformemente. Cada una empieza
con factor 0,5 e incluye la razón de Hastings de la escala. Usan la misma
regla de ajuste que los demás movimientos de escala.

La configuración predeterminada no usa las mezclas adaptativas de ORC, los
kernels Bactrian ni los movimientos de época o de estiramiento del árbol.
Rove también omite la propuesta de tasa de mutación
del XML, sin efecto en una dimensión. Igualar el modelo probabilístico no
establece la misma mezcla de la cadena ni el mismo ESS por segundo.

El trazado añade `gamma_shape`, `gamma_prior`, `clock_sigma`,
`clock_rate_prior`, `clock_sigma_prior` y `mean_branch_rate` (media aritmética
sin ponderar). Los árboles conservan longitudes temporales y anotan cada
rama con `[&rate=...]`; longitud por tasa da la distancia antes del
multiplicador del carácter. Las alturas son relativas al multiplicador fijo
del reloj, sin calibración de calendario. `run.json` registra la configuración,
priors, valores iniciales, mapeo y tasas finales. No es un checkpoint para
reanudar una cadena.

## Propuestas conjuntas

Tres movimientos adicionales están disponibles mediante `--weight`, con peso
cero por defecto. Requieren el reloj relajado. Cambian las propuestas y
mantienen la verosimilitud, los priors y las coordenadas de los parámetros.

- `node_time_rates` elige uniformemente la altura de un nodo interno distinto
  de la raíz, entre su hijo más antiguo y su padre. Cada tasa incidente pasa
  a `tasa × duración antigua / duración nueva`, conservando la distancia.
  El logaritmo de la razón de Hastings suma `ln(duración antigua / duración
  nueva)` de las tres ramas.
- `root_time_rates` multiplica la altura de la raíz por `f`, uniforme en
  `[s, 1/s]` (`s = 0,75` inicialmente), y compensa sus dos tasas incidentes.
  Su logaritmo de Hastings añade `−ln(f)` a los términos de duración.
- `clock_sigma_rates` multiplica sigma por el mismo tipo de factor
  (`s = 0,5` inicialmente), conservando cada tasa logarítmica estandarizada
  `z = (ln(tasa) + σ²/2) / σ`. Con `d` tasas, el logaritmo de Hastings es
  `(d − 1) ln(f) + Σ ln(tasa nueva / tasa antigua)`.

Estos factores corrigen el cambio de volumen de la propuesta; no son un
prior adicional. El jacobiano de las tasas para sigma es
`f^d × Π(tasa nueva / tasa antigua)` y el multiplicador uniforme aporta
`1/f`. La inversa usa `1/f` y la altura antigua. Los valores inválidos se
rechazan antes de modificar el estado. Las distancias pasan por el mecanismo
existente de actualización y restauración de caché.

Los tests comprueban inversas, jacobianos numéricos, distribuciones conocidas,
cambios del índice de la raíz, restauración exacta y concordancia entre backends.
Son métodos establecidos para relojes relajados, descritos por
[Douglas, Zhang y Bouckaert (2021)](https://doi.org/10.1371/journal.pcbi.1008322);
esta implementación no reproduce todo ORC. El
[protocolo de evaluación](https://github.com/getcortexlabs/rove/blob/main/fixtures/validation/SAMPLING-EFFICIENCY.md)
separa la selección de la candidata de la validación con cuatro cadenas y
define las condiciones para cambiar la configuración predeterminada.

## Validación

`tests/rates.rs` reconstruye ambos estados archivados de BEAST 2.7.8 a partir
de alturas, forma gamma, sigma y tasas explícitas. La mayor diferencia
absoluta de log-verosimilitud es inferior a 8e-10; cada componente del prior
coincide dentro de 2e-12. Rove calcula las categorías, no las importa de BEAST.

Rove invierte la función de distribución gamma mediante [su serie positiva](https://dlmf.nist.gov/8.7.E1)
y pasos de Newton acotados en espacio logarítmico. BEAST usa su aproximación
heredada AS91. La mayor diferencia de tasas en la cuadrícula archivada de
20 formas es inferior a 1,4e-8; esto explica una tolerancia mayor que en las
pruebas que importan las tasas. Las fórmulas cerradas exponencial y Erlang
aportan verificaciones numéricas independientes.

Otras pruebas comprueban los hiperpriors muestreando sin datos, el mapeo con
distintas raíces, cálculos almacenados frente a evaluaciones nuevas en cada
paso, restauración tras rechazo, muestras idénticas entre backends y ausencia
de asignaciones durante los pasos probados.

Los resúmenes de posterior y las frecuencias de clados cumplieron los criterios fijados
antes de las ejecuciones en seis casos construidos con 20 taxones, incluidos homoplasia,
datos ausentes y señal reducida. El
[informe](https://github.com/getcortexlabs/rove/blob/main/benchmarks/inference/2026-10-29/README.md)
registra todos los lotes, incluida la extensión de Mkv exigida por el criterio de ESS,
tiempos y ESS por segundo. Rove terminó los cinco casos de Mk sin corrección en menos
tiempo, pero BEAST obtuvo más ESS por segundo para la densidad log-posterior y el sigma
del reloj. Las mejoras de las propuestas se siguen en
[#27](https://github.com/getcortexlabs/rove/issues/27).

Seis estados fijos adicionales cubren estos casos. Con tasas de categoría importadas, la
diferencia máxima de verosimilitud fue 1,14e-13; al generar las categorías de forma
independiente, llegó a 8,16e-8, por la misma diferencia de aproximación de cuantiles
descrita arriba.
