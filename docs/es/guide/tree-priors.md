---
description: "Priors de árbol disponibles, sus supuestos y los límites de cada opción."
---

# Elegir el prior del árbol

El prior del árbol describe la ramificación antes de considerar los caracteres.
Es distinto del prior de sigma del reloj (variación entre ramas) y del prior
de la forma gamma (variación entre caracteres).

## Opciones disponibles · 5 de noviembre de 2026

**Yule está disponible como prior opcional para especies contemporáneas.** Birth–death sigue siendo el predeterminado y la datación de fósiles usa FBD. Elige según el muestreo y los supuestos biológicos del estudio; implementarlo no demuestra que ajuste mejor las matrices de Thiago. Consulta el [modelo y sus controles](./reference-model).

| Análisis | Modelo | Estado y límite en Rove |
| --- | --- | --- |
| Especies muestreadas al mismo tiempo | Birth–death | Disponible; puntas en cero, muestreo completo, tasas constantes |
| Mismo caso, suponiendo ausencia de extinción | Yule | Disponible; nacimiento puro, sin extinción |
| Fósiles con edades | Fossilized birth–death (FBD) | Disponible; proceso fijo o estimado, edades inciertas; fósiles terminales |
| Fósiles que podrían ser ancestros directos | FBD con ancestros muestreados | Trabajo separado en #52 |
| Muestras dentro de una población | Coalescente / skyline demográfico | Pospuesto; no sustituye la diversificación de especies |
| Morfología sin reloj ni datación | Topología uniforme y prior de longitudes de ramas | Evaluación separada; requiere otra parametrización |
| Diversificación o recuperación fósil variable en el tiempo | Skyline birth–death/FBD | Pospuesto hasta definir el estudio y disponer de información suficiente |

La [guía de Yule de BEAST](https://beast2-dev.github.io/hmc/hmc/YuleModel/index/)
lo limita a puntas contemporáneas. El
[tutorial morfológico de RevBayes](https://revbayes.github.io/tutorials/morph_tree/V2)
muestra una alternativa sin reloj. Su
[tutorial FBD](https://revbayes.github.io/tutorials/fbd_simple/)
modela conjuntamente diversificación y recuperación de fósiles.

Un fósil sin edad no se convierte en una especie actual. Poner todas las puntas
en cero es una hipótesis, no una solución para edades ausentes. El muestreo,
la extinción y la escala temporal deben corresponder al análisis. Ni el tamaño
de la matriz ni un ESS alto eligen el prior adecuado.

Nuestro birth–death usa la convención Gernhard sin escala, condicionada al número de puntas, no a raíz/origen. FBD condiciona en el origen, sin condicionar en supervivencia/muestreo actual; la probabilidad de eliminación es cero. Los procesos fijos y estimados registran convenciones de densidad diferentes. Véase [Datación de fósiles](./tip-dating) antes de comparar modelos.

## Implementación y validación de Yule

La implementación sigue `YuleModel` de BEAST 2.7.8: muestreo completo,
condicionado al número de puntas, sin condicionar en raíz/origen, sin puntas
fósiles ni calibraciones de nodos. Para n puntas, tasa λ y alturas internas h,
la log-densidad sin factor de escala es
`(n−1) log(λ) − λ (sum(h) + altura_de_la_raíz)`.
Es el caso de extinción cero de nuestro birth–death;
véase [Gernhard (2008)](https://doi.org/10.1016/j.jtbi.2008.04.005).

La validación incluye 24 estados fijos de BEAST 2.7.8, simulaciones independientes del prior, el límite birth–death sin extinción, cuadratura de posteriores de potencia y reanudación idéntica por checkpoint. Se omiten el parámetro y las propuestas de extinción. La app y el CLI conservan la elección en configuración, Review/History, predicción y comparación de modelos.

El prior de natalidad es Uniform(0,U), propio, con U positivo configurable. U=1000 es una referencia en unidades de altura del reloj unitario, no una recomendación biológica universal. Revisa ejecuciones solo del prior y límites alternativos defendibles. Las comparaciones marginales requieren los mismos datos, restricciones y convenciones de normalización compatibles; consulta los [detalles del modelo](./reference-model).

Comparar cadenas independientes, muestreo solo del prior y sensibilidad de
clados y alturas. Mayor velocidad o soporte no demuestra superioridad biológica.

## ORC es otra decisión

Rove tiene tasas lognormales explícitas y propuestas conjuntas opcionales,
pero no todos los operadores de ORC. La
[auditoría del XML](https://github.com/getcortexlabs/rove/blob/main/fixtures/reference/20x50/orc-audit.md)
detalla versiones, correspondencias y pruebas. El esquema predeterminado se
mantiene mientras #48 y #72 evalúan pesos. El intercambio conjunto de topología y tasas de [#97](https://github.com/getcortexlabs/rove/issues/97) está implementado como experimento opcional para reloj relajado sin datación, con peso cero por defecto. Su beneficio comparativo en morfología sigue sin medirse; [#128](https://github.com/getcortexlabs/rove/issues/128) detalla la cobertura parcial de ORC y los operadores y validaciones pendientes.
