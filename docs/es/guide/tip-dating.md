---
description: "Date terminales fósiles con edades fijas o intervalos, una escala temporal explícita y un prior de nacimiento-muerte fosilizado."
---

# Datación de terminales fósiles

`rove sample --dating settings.json` añade edades fósiles y una escala temporal al análisis morfológico. Admite relojes estricto y relajado, particiones, condicionamiento por selección de caracteres, cadenas independientes y checkpoints. La configuración está disponible en la app nativa del Mac y en el CLI Rust. No necesita BEAST ni Java.

## En la app del Mac

1. Importe la matriz, abra **Model** y active **Date fossils** en **A place in time**. La interfaz está disponible en inglés y portugués.
2. Elija la unidad temporal y abra **Edit tip ages**. Todos los terminales empiezan como actuales (cero). Busque cada fósil y elija una edad fija o un intervalo de menor a mayor antigüedad; pulse **Apply ages**. El editor requiere límites distintos; si son iguales, use **Fixed age**.
3. Para cada parámetro del proceso, elija **Keep fixed** o **Estimate**. Introduzca un valor fijo, o un valor inicial y un prior. Las tasas usan una media exponencial; el muestreo actual usa los parámetros beta α y β; el origen usa una media del tiempo adicional anterior a la mayor edad permitida. Los campos empiezan vacíos. Los ejemplos comprueban el software; no son recomendaciones biológicas.
4. Elija **Estimate** para el reloj global (tasa inicial y media del prior exponencial) o **Keep fixed** (solo la tasa). Las tarjetas del reloj evolutivo siguen eligiendo tasas estrictas o relajadas entre ramas.
5. Revise la configuración en **Review** e inicie el análisis. El historial conserva edades, unidad, parámetros y modo del reloj. También se preservan al guardar la configuración o reanudar una ejecución.

Se aceptan punto o coma decimal y notación científica, sin separadores de miles. Cambiar el nombre de la unidad no convierte los valores. Desactivar la datación recupera el modelo sin fechas y conserva los valores introducidos para reactivarla; importar otra matriz inicia una configuración nueva.

La escala del árbol anotado muestra la unidad registrada. La comparación de cadenas identifica los parámetros fijos verificados como **Fixed**: ESS y R-hat no se aplican a valores que nunca se muestrearon.

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

Nacimiento, muerte y muestreo fósil son tasas por linaje y unidad de tiempo. present_sampling es una probabilidad. origin es la edad inicial del linaje ancestral, anterior a la raíz y a todas las edades terminales permitidas. En este ejemplo, estos cinco valores permanecen fijos. Los priors opcionales siguientes permiten estimarlos individualmente. Deben elegirse según la pregunta científica y examinar su sensibilidad; los valores del ejemplo no son valores biológicos predeterminados.

Con clock_prior_mean, clock_rate es el valor inicial y el reloj se estima con un prior exponencial de esa media. Omita clock_prior_mean para fijar clock_rate. Use `--model reference` para categorías gamma y reloj relajado, o `--sample-from-prior` para excluir la verosimilitud de caracteres. El número de muestras, la información de edad y las restricciones impuestas siguen informando los parámetros estimados.

## Parámetros del proceso fijos o estimados

Añada estos dos campos al JSON de datación para estimar los cinco parámetros. Es un fragmento, no una configuración completa. La versión lista para ejecutar está en `fixtures/validation/fbd-estimation/dating.json`:

```json
{
  "process_model": "joint_sampling_v1",
  "process_priors": {
    "birth_rate_mean": 0.08,
    "death_rate_mean": 0.03,
    "fossil_sampling_rate_mean": 0.02,
    "present_sampling_beta": [2, 2],
    "origin_excess_mean": 30
  }
}
```

Omita el prior de un parámetro para mantenerlo fijo; conserve al menos uno en `process_priors`. Se requieren tanto el identificador explícito del modelo como el objeto de priors. Los valores correspondientes al principio del JSON pasan a ser valores iniciales. Los archivos existentes sin estos dos campos conservan el proceso fijo.

Las medias exponenciales deben ser positivas y finitas. Las tasas estimadas empiezan por encima de cero; la muerte fija puede ser cero. Los parámetros beta deben estar en [0.001, 1000], con probabilidad inicial estrictamente entre cero y uno. La media del prior beta es α / (α + β).

`origin_excess_mean` es la media de una exponencial sobre **el origen menos la mayor edad terminal permitida**. Aquí el límite es 12 Ma: un exceso medio de 30 Ma da una media nominal del prior de origen de 42 Ma. El origen también debe ser anterior a la raíz actual. Este soporte conjunto se impone sin renormalizar el prior del origen para cada altura de la raíz.

Tasas, tiempo, origen y reloj pueden compensarse entre sí. Estimarlo todo puede producir posteriores amplias, correlacionadas y más lentas de explorar. Examine traces, diagnósticos entre cadenas y sensibilidad a priors justificados científicamente; un ESS alto no demuestra que los datos identifiquen todos los parámetros.

## Supuestos del prior

El FBD incluye nacimiento, extinción, muestreo fósil y muestreo incompleto actual. Los árboles se restringen a **fósiles terminales**, sin ancestros muestreados. Es una restricción impuesta, no evidencia recuperada por Rove. El muestreo no obliga al linaje a morir: la probabilidad de eliminación es cero.

Con todos los parámetros del proceso fijos, la densidad restringida del árbol tiene un normalizador constante omitido. Estimar selecciona explícitamente `joint_sampling_v1`: el número observado de especímenes, las edades y las restricciones terminales y de clados informan tanto el proceso como el árbol. Rove multiplica la densidad de muestreo FBD por priors propios y factores de observación. La masa del espacio restringido depende de los parámetros y se conserva como información; **no** se supone que se cancele ni se divide por ella para cada valor de los parámetros. Esto difiere de un prior de árbol condicionado y normalizado para cada proceso.

La densidad FBD se condiciona al origen, sin condicionamiento a supervivencia ni muestreo actual, coincidiendo con [BEAST 2 SA](https://www.beast2.org/xml/sa.evolution.speciation.SABirthDeathModel.html) en estados validados. La [derivación y validación](https://github.com/getcortexlabs/rove/tree/main/fixtures/validation/fbd-estimation) especifica el modelo conjunto. Verosimilitudes marginales, ancestros muestreados y rangos de aparición siguen sin admitirse. Edades y clados impuestos no son recuperación independiente de los caracteres.

## Archivos y comprobaciones

El trace contiene fossilized_birth_death, clock_rate, dating_parameter_prior y tip_age_1, tip_age_2, etc., en orden de la matriz. Las columnas gamma y del reloj relativo permanecen cuando se activan. Los parámetros fijos tienen ESS no disponible; la comparación los marca fixed_by_model y excluye únicamente valores verificados como fijos de la parada automática. Un parámetro estimado que se vuelva constante sigue impidiendo la parada.

`run.json` conserva la configuración completa y su huella. Los checkpoints incorporan la configuración: reanudar no requiere el JSON original. Los árboles conservan alturas explícitas y duraciones; las tasas relativas anotadas no incluyen el reloj global, guardado en el trace. `rove summarize`, `inspect-tree`, `diagnose`, `compare` y `predict` admiten estas salidas. Los resúmenes conservan la unidad cuando existe el registro de ejecución. Un archivo de árbol aislado no identifica por sí solo la unidad física.

La validación compara ocho densidades fijas con BEAST 2.7.8 / SA 2.1.1, comprueba distribuciones pequeñas del prior mediante un límite analítico y cuadratura independiente de ecuaciones diferenciales, y verifica restauración de verosimilitud, cambio de unidad, reanudación, exportaciones y propuestas sin asignaciones de memoria. Estas comprobaciones no establecen convergencia ni rendimiento con fósiles empíricos. El [material de validación](https://github.com/getcortexlabs/rove/tree/main/fixtures/validation/dating) registra fórmulas, fuentes, versiones y comandos.

Al estimar algún parámetro del proceso, el trace añade `fbd_birth_rate`, `fbd_death_rate`, `fbd_fossil_sampling_rate`, `fbd_present_sampling` y `fbd_origin`. Los miembros fijos también se registran y verifican en las comparaciones. `run.json` guarda `dating_density_convention` y los priors completos. Las configuraciones nativas usan formato 4 para estimar el proceso, formato 3 para datación con proceso fijo y formato 2 sin fechas; las apps anteriores rechazan formatos desconocidos. Reanudar checkpoints requiere el mismo ejecutable del motor.

La validación añade ocho estados completos del prior frente a BEAST 2.7.8 / SA 2.1.1, condicionales analíticas, integración conjunta independiente, combinaciones fijas/estimadas y reanudación exacta. El [informe con datos construidos](https://github.com/getcortexlabs/rove/tree/main/benchmarks/fbd-estimation/2026-10-30) registra tiempo, ESS, R-hat, diferencias de clados y correlaciones. No establece precisión, identificabilidad ni rendimiento con fósiles empíricos.

El [estudio de frecuencia gamma](./reference-model#frecuencia-de-las-propuestas-gamma) identifica la baja frecuencia de actualización como una limitación del muestreo en este ejemplo construido. Un ajuste evaluado mediante la CLI mejora la ESS gamma, pero no cumple todos los criterios de eficiencia para convertirse en predeterminado. La app nativa conserva la configuración existente.
