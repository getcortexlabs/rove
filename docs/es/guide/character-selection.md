---
description: "Elige Mk, Mkv o Mkparsinf según cómo se recopilaron los caracteres morfológicos."
---

# Selección de caracteres

La corrección debe corresponder a los caracteres que podían entrar en la matriz. No es un ajuste de velocidad o calidad.

| Caracteres conservados durante la recopilación | Opción de CLI | App de Mac |
| --- | --- | --- |
| Constantes y variables | `--conditioning none` | All characters · Mk |
| Solo variables | `--conditioning variable` (predeterminado) | Variable only · Mkv |
| Solo informativos para parsimonia | `--conditioning informative` | Informative only · Mkparsinf |

Un carácter completo es informativo para parsimonia cuando al menos dos estados distintos aparecen, cada uno, en dos o más taxones. `0011` cumple; `0001` y `00123` no. Con varios estados, excluir solo constantes y patrones con un único terminal diferente no basta.

Mkparsinf funciona con ambos relojes, particiones por número de estados, cadenas independientes y checkpoints. Cada partición usa Mk no ordenado de frecuencias iguales. Rove divide la verosimilitud observada por la probabilidad de un carácter informativo en el mismo árbol y con las mismas tasas de rama, después de integrar las categorías de tasa. Almacena probabilidades por subárbol en lugar de enumerar las columnas excluidas. Esta recurrencia no se aplica a modelos ordenados o de frecuencias desiguales.

Las celdas faltantes e inciertas se aceptan solo si toda forma permitida de completarlas es informativa: `0011?` cumple, `001??` no. El condicionamiento considera el carácter completo en todos los taxones, no un evento separado para cada máscara de datos faltantes. `rove inspect` informa `informative_error`; la app bloquea análisis incompatibles y la CLI identifica el número original del carácter. Rove nunca elimina caracteres automáticamente. Se rechazan árboles extremadamente cortos cuya probabilidad de retención no pueda distinguirse de cero en doble precisión.

En la app, abre **Model → Which characters did you include?** y elige **Informative only · Mkparsinf** solo si corresponde a la recopilación. El ejemplo incluido de 20 × 50 contiene caracteres no informativos; conserva Mkv para ese ejemplo.

Para una comprobación breve del funcionamiento:

```sh
target/release/rove sample fixtures/validation/informative/matrix.nex \
  --conditioning informative --model reference \
  --output runs/informative --seed 42 --steps 10000 --sample-every 100
```

Esta ejecución breve comprueba el flujo, no la convergencia. La [validación](https://github.com/getcortexlabs/rove/tree/main/fixtures/validation/informative) incluye enumeración completa de patrones, datos faltantes, restauración de checkpoints y concordancia en estados fijos con BEAST 2.7.8. No demuestra concordancia posterior ni una mejora de velocidad de inferencia.
