# La carpeta de tu ejecución

Abre **What are these files?** junto a la carpeta en Resultados o History para consultar la ayuda sin conexión. Una ejecución con una cadena suele tener esta estructura:

```text
analysis-<id>/
  setup.rove.json
  session.json
  events.jsonl
  engine.stderr.log
  results/
    run.json
    trace.log
    trees.nexus
    checkpoint.json
    checkpoint-<step>.bin
```

| Archivo | Uso |
| --- | --- |
| `setup.rove.json` | Configuración solicitada, ruta y huella de la matriz. Permite preparar otra ejecución. |
| `session.json` | Sesión, estado y detalles de ejecución de la aplicación. |
| `events.jsonl` | Eventos de progreso, un objeto JSON por línea. |
| `engine.stderr.log` | Mensajes del motor para investigar problemas; no es la traza numérica. |
| `dating.json` | Datos de datación de fósiles, cuando está activada. |
| `results/run.json` | Configuración del motor, huellas de compilación y entrada, y registro de finalización. |
| `results/trace.log` | Valores numéricos muestreados. Abre Diagnostics en Rove o usa Tracer. |
| `results/trees.nexus` | Árboles muestreados sin resumir, incluido el estado inicial. |
| `results/checkpoint.json` y `checkpoint-<step>.bin` | Índice y estado binario para reanudar. Conserva el binario indicado y los resultados asociados. |

Con varias cadenas, cada una guarda sus archivos en `results/chain-01/`, `chain-02/`, etc. El registro del grupo permanece en `results/run.json`.

## De las muestras al árbol

Prueba la matriz pública de ejemplo de 20 × 50 incluida en la aplicación. Tras ejecutar, revisa **Diagnostics** y compara ejecuciones independientes. Elige **Summarize trees**, decide cuánto del inicio descartar (burn-in) y abre `summaries/summary-<id>/mcc.tree` en Rove o FigTree. Es un árbol representativo anotado. `samples.trees` contiene muestras sin resumir; `summary.json` registra las opciones y el cálculo del soporte. Resumir o completar los pasos solicitados no demuestra convergencia.

`.nexus` y `.trees` pueden nombrar muestras de árboles NEXUS. Cambiar la extensión no convierte el archivo ni calcula soporte de los nodos.

## Informes posteriores

Las carpetas opcionales `diagnostics/`, `summaries/`, `comparisons/`, `predictions/` y `marginal/` aparecen al usar las herramientas correspondientes. Los diagnósticos incluyen `diagnostics.json`. Una [comprobación predictiva](./posterior-prediction) incluye `predictive.json`, `replicates.jsonl`, `characters.tsv` y matrices simuladas en `matrices/`. Guarda los informes con su ejecución de origen. Las muestras de árboles importadas usan `tree-source.json` para registrar su ruta original y huella.

## Conservar o compartir

Para reanudar, conserva la carpeta completa, la matriz original sin cambios y la misma compilación de Rove. La aplicación **no** copia la matriz ni el motor a esta carpeta. Un JSON de configuración o informe no sustituye el checkpoint binario. Si se seleccionó un subconjunto, conserva la matriz exacta registrada en la configuración.

Para una figura, comparte `mcc.tree` y las opciones del resumen. Para revisar muestreo, incluye traza y árboles originales. Para reproducir el análisis, conserva todo el contexto y las opciones del modelo. Revisa rutas y datos sensibles antes de compartir. Las ejecuciones parciales o interrumpidas pueden tener resultados incompletos; comprueba su estado antes de interpretarlos.
