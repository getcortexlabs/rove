---
description: "La matriz de control sintética que acompaña a Rove, el script que la regenera y la degrada, y cómo ejecutarla en BEAST 2."
---

# Matrices sintéticas

`fixtures/synthetic/control_20x50.nex` es un control positivo: 20 taxones, 50 caracteres binarios, sin homoplasia, sin datos faltantes, árbol conocido. T20 es el grupo externo, todo ceros. El árbol del grupo interno está balanceado y cada uno de sus 18 clados recibe sinapomorfías exclusivas, tres para los nueve clados más grandes y dos para el resto. Los últimos cinco caracteres son autapomorfías. Esta construcción no garantiza soporte posterior: el enraizamiento, los modelos de reloj y los priors también afectan al resultado.

`control_20x50.tre` es el árbol verdadero en Newick, repetido también como comentario al final del archivo NEXUS. `control_20x50.legend.txt` asocia cada carácter al clado que sustenta.

## Regenerarla y degradarla

`tools/synth_matrix.py` regenera el control y produce variantes degradadas:

```sh
python3 tools/synth_matrix.py --out fixtures/synthetic/control_20x50
python3 tools/synth_matrix.py --homoplasy 10 --seed 7 --out /tmp/homoplasy10
python3 tools/synth_matrix.py --missing 0.2 --out /tmp/missing20
python3 tools/synth_matrix.py --drop T1-T4 --out /tmp/no_t1_t4
```

Las degradaciones se listan al final del archivo de leyenda.

## Ejecutarla en BEAST 2

En BEAUti, instala el paquete MorphModels, importa el archivo NEXUS y elige el modelo Mk de Lewis con la corrección de ascertainment para caracteres variables, un reloj estricto y un prior de árbol de Yule.

Esta es una configuración de ejemplo para el control generado. Difiere del análisis de referencia seleccionado que se describe abajo.

## Matriz de referencia incluida

`fixtures/reference/20x50/matrix.nex` es un conjunto de datos separado, con su leyenda original y su XML de BEAST. Su topología y la asignación de caracteres difieren del control generado. El experimento documentado con BEAST 2.7.8 usa un reloj relajado, cuatro categorías gamma, un prior de nacimiento-muerte y una restricción de monofilia del grupo interno. Las entradas, la procedencia y los resultados están en el [informe de referencia](https://github.com/getcortexlabs/rove/blob/main/benchmarks/beast-2.7.8/2026-10-25/README.md).

El [lector de NEXUS](./nexus) importa ambos fixtures de forma independiente. Cada uno tiene 23 patrones observados que representan 50 caracteres, pero los patrones difieren. Ninguna matriz sustituye a la otra en la validación.
