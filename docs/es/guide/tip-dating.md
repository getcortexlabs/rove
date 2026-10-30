---
description: "Date terminales fósiles con edades fijas o intervalos, una escala temporal explícita y un prior de nacimiento-muerte fosilizado."
---

# Datación de terminales fósiles

`rove sample --dating settings.json` añade edades fósiles y una escala temporal al análisis morfológico. Admite relojes estricto y relajado, particiones, condicionamiento por selección de caracteres, cadenas independientes y checkpoints. La configuración está disponible en el CLI Rust; la pantalla de preparación del Mac todavía no tiene controles de datación. No necesita BEAST ni Java.

## Edades y unidades

La edad se mide **hacia atrás desde el presente**: cero representa un taxón actual; 12 significa 12 unidades antes del presente. En el ejemplo, `time_unit: "Ma"` significa millones de años. Edades, alturas, origen y duraciones de ramas deben usar la misma unidad. Rove registra su nombre, pero no convierte unidades.

La tasa del reloj expresa cambios esperados por carácter y unidad de tiempo. Los cambios esperados de una rama son duración × tasa global del reloj × tasa relativa de la rama. El reloj relajado añade tasas relativas cuya distribución tiene media uno; las categorías gamma multiplican además la tasa del carácter. Sin datación, los modelos existentes mantienen reloj unitario y terminales de edad cero.

Cada taxón necesita una edad explícita. Un número la fija; `[mínimo, máximo]` permite estimarla dentro del intervalo. Los intervalos positivos representan incertidumbre en la **edad de un espécimen**, no el rango entre la primera y última aparición de una especie. Esta distinción sigue el [tutorial de especímenes de RevBayes](https://revbayes.github.io/tutorials/fbd/fbd_specimen). No pueden incluir el presente; `[12,12]` es fijo. Los nombres deben coincidir con las etiquetas NEXUS interpretadas, donde un guion bajo sin comillas se convierte en espacio.

## Pruebe el ejemplo construido

```sh
cargo run --release --bin rove -- sample fixtures/validation/dating/matrix.nex \
  --dating fixtures/validation/dating/dating.json \
  --output runs/dated-example --seed 25 --steps 100000 --sample-every 100
```

El ejemplo comprueba el software; no es un análisis biológico ni garantiza convergencia. Su JSON contiene:

```json
{
  "time_unit": "Ma",
  "birth_rate": 0.08,
  "death_rate": 0.03,
  "fossil_sampling_rate": 0.02,
  "present_sampling": 0.7,
  "origin": 50,
  "clock_rate": 0.01,
  "clock_prior_mean": 0.01,
  "ages": {
    "Living A": 0,
    "Fossil B": [5, 9],
    "Living C": 0,
    "Fossil D": 12
  }
}
```

Nacimiento, muerte y muestreo fósil son tasas por linaje y unidad de tiempo. present_sampling es una probabilidad. origin es la edad inicial del linaje ancestral, anterior a la raíz y a todas las edades terminales permitidas. Estos cinco valores permanecen fijos. Deben elegirse según la pregunta científica y examinar su sensibilidad; los valores del ejemplo no son valores biológicos predeterminados.

Con clock_prior_mean, clock_rate es el valor inicial y el reloj se estima con un prior exponencial de esa media. Omita clock_prior_mean para fijar clock_rate. Use `--model reference` para categorías gamma y reloj relajado, o `--sample-from-prior` para examinar el prior conjunto sin verosimilitud de caracteres.

## Supuestos del prior

El proceso FBD incluye nacimiento, extinción, muestreo fósil y muestreo incompleto en el presente. Esta implementación restringe los árboles a **fósiles terminales**, sin ancestros muestreados. Muestrear un fósil no obliga al linaje a morir: la probabilidad de eliminación es cero. El origen y los parámetros del proceso permanecen fijos para que la normalización del espacio restringido sea constante durante el muestreo.

La log-densidad coincide en los estados validados con la convención de origen fijo, sin condicionamiento a la supervivencia, de [BEAST 2 SA](https://www.beast2.org/xml/sa.evolution.speciation.SABirthDeathModel.html). Es una **densidad restringida no normalizada**, inadecuada para verosimilitudes marginales o factores de Bayes. Esta versión no estima parámetros del proceso ni origen, no muestrea ancestros ni modela rangos de aparición de especies. Recuperar un intervalo impuesto no constituye evidencia independiente de la edad fósil.

## Archivos y comprobaciones

El trace contiene fossilized_birth_death, clock_rate, dating_parameter_prior y tip_age_1, tip_age_2, etc., en orden de la matriz. Las columnas gamma y del reloj relativo permanecen cuando se activan. Los parámetros fijos tienen ESS no disponible; la comparación los marca fixed_by_model y excluye únicamente valores verificados como fijos de la parada automática. Un parámetro estimado que se vuelva constante sigue impidiendo la parada.

`run.json` conserva la configuración completa y su huella. Los checkpoints incorporan la configuración: reanudar no requiere el JSON original. Los árboles conservan alturas explícitas y duraciones; las tasas relativas anotadas no incluyen el reloj global, guardado en el trace. `rove summarize`, `inspect-tree`, `diagnose`, `compare` y `predict` admiten estas salidas. Los resúmenes conservan la unidad cuando existe el registro de ejecución. Un archivo de árbol aislado no identifica por sí solo la unidad física.

La validación compara ocho densidades fijas con BEAST 2.7.8 / SA 2.1.1, comprueba distribuciones pequeñas del prior mediante un límite analítico y cuadratura independiente de ecuaciones diferenciales, y verifica restauración de verosimilitud, cambio de unidad, reanudación, exportaciones y propuestas sin asignaciones de memoria. Estas comprobaciones no establecen convergencia ni rendimiento con fósiles empíricos. El [material de validación](https://github.com/getcortexlabs/rove/tree/main/fixtures/validation/dating) registra fórmulas, fuentes, versiones y comandos.
