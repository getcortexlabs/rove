---
description: "Estime log-verosimilitudes marginales a partir de posteriores de potencia, con stepping stone y path sampling, y compare modelos con factores de Bayes."
---

# Comparación de modelos

`rove marginal` estima la log-verosimilitud marginal de un modelo configurado: la probabilidad de la matriz bajo el modelo, integrando todos los árboles y parámetros. La diferencia entre las log-verosimilitudes marginales de dos modelos es el logaritmo del factor de Bayes. La estimación procede de posteriores de potencia: el objetivo de la cadena con la verosimilitud elevada a un exponente entre cero, el prior, y uno, la posterior. El enfoque sigue el [tutorial de selección de modelos de RevBayes](https://revbayes.github.io/tutorials/model_selection_bayes_factors/bf_intro) y el PathSampler del paquete MODEL_SELECTION de BEAST 2. No se requiere software externo para ejecutarlo.

El comando está disponible en el CLI en Rust. La app para Mac no lo expone.

## Ejecutar una estimación

```sh
cargo run --release --bin rove -- marginal fixtures/reference/20x50/matrix.nex \
  --output runs/marginal-mkv --seed 1 --steps 1000000 --sample-every 1000 \
  --powers 32 --alpha 0.3 --burnin-percent 20 \
  --monophyly T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11,T12,T13,T14,T15,T16,T17,T18,T19
```

Se aplican las opciones de modelo de `rove sample`: `--model`, `--conditioning`, `--partitioning`, `--gaps`, `--dating`, `--monophyly`, `--weight`, `--tuning`, `--no-tuning` y `--backend`. Para comparar modelos, ejecuta el comando una vez por modelo sobre la misma matriz, con las mismas restricciones, y resta las estimaciones. `--sample-from-prior`, las cadenas paralelas, los checkpoints y los diagnósticos de parada no están disponibles.

`--powers` es el número de exponentes de la verosimilitud, incluidos cero y uno. Son cuantiles de una distribución Beta(α, 1) con espaciado uniforme, `β_i = (i / (powers − 1))^(1/α)`; un `--alpha` menor concentra más exponentes cerca de cero, donde la media de la log-verosimilitud cambia más rápido. Los valores anteriores siguen las convenciones del PathSampler de BEAST 2. Cada potencia ejecuta `--steps` propuestas y registra cada `--sample-every`; el primer `--burnin-percent` de las muestras de cada potencia, por recuento, se descarta. La cadena va de la potencia uno a cero, y cada potencia continúa desde el estado final de la anterior, conservando el ajuste de las propuestas.

## Salida

El directorio de salida contiene `run.json`, con el modelo, el esquema de potencias y los recuentos de operadores, `marginal.json`, con las estimaciones, y un `power-NN/trace.log` por potencia. Esos traces llevan las columnas de posterior, verosimilitud y prior sin el exponente aplicado, como un trace común; el exponente solo entró en la razón de aceptación. No se escriben árboles.

`marginal.json` informa dos estimaciones con errores estándar:

- **Stepping stone** ([Xie et al. 2011](https://doi.org/10.1093/sysbio/syq085)) suma, sobre potencias adyacentes, el log de la media de `L^(β_k − β_{k−1})` bajo la potencia inferior. Su error estándar suma la varianza de cada término por el método delta, usando el tamaño efectivo de muestra de los pesos de ese término.
- **Path sampling** ([Lartillot y Philippe 2006](https://doi.org/10.1080/10635150500433722)) integra la media de la log-verosimilitud sobre el exponente con la regla del trapecio. Su error estándar suma la varianza de cada media a partir de su tamaño efectivo de muestra. El error de la propia regla del trapecio no se incluye, así que las dos estimaciones pueden diferir en más que sus errores estándar cuando el esquema es grueso.

Para cada potencia, el archivo registra el número de muestras retenidas, la media y la desviación estándar de la log-verosimilitud, su ESS y si se alcanzó el límite de retardo de la autocorrelación. Una ESS baja cerca de la potencia cero es habitual: esas cadenas exploran el prior, donde los árboles se mueven libremente y la verosimilitud varía mucho. Más pasos por potencia, o más potencias, reducen el error.

## Cómo leer el resultado

Siguiendo a [Kass y Raftery (1995)](https://doi.org/10.1080/01621459.1995.10476572), un log del factor de Bayes por encima de unos 3 a 5 suele leerse como soporte fuerte al primer modelo. Compara solo estimaciones de la misma matriz, los mismos taxones y las mismas restricciones, y repite la estimación con otras semillas antes de leer una diferencia menor que unos pocos errores estándar. La verosimilitud marginal depende de los priors, incluidos los de nacimiento–muerte y del reloj, así que un factor de Bayes compara los modelos junto con sus priors.

Mk y Mkv definen verosimilitudes para procesos de observación distintos: Mkv condiciona a que todo carácter sea variable. Un factor de Bayes entre ellos compara esos procesos además de los modelos; consulta [selección de caracteres](./character-selection).

Con datación de fósiles y proceso fijo, la densidad FBD omite un normalizador que depende de la configuración del proceso, de los terminales y de los límites de edad. Compara modelos datados solo cuando esa configuración sea idéntica, para que la constante se cancele. Con parámetros del proceso estimados, el objetivo conjunto de muestreo incluye factores de observación cuya normalización no se ha derivado para comparar modelos; esas comparaciones no se admiten. Consulta [datación de fósiles](./tip-dating).

## Validación

Las pruebas comprueban ambos estimadores en un modelo gaussiano conjugado con muestras exactas, incluida la calibración de sus errores estándar en cuarenta réplicas, y frente a una cuadratura en un árbol de tres taxones con una altura de nodo libre, calculada con las densidades del propio motor. Una cadena en la potencia cero reproduce paso a paso la cadena que muestrea el prior, evaluando aún la verosimilitud; los exponentes fuera de [0, 1] se rechazan, y los checkpoints registran el exponente en uso. La [comparación con BEAST 2.7.8](https://github.com/getcortexlabs/rove/blob/main/benchmarks/marginal-likelihood/2026-10-31/README.md) ejecutó el PathSampler de MODEL_SELECTION 1.6.2 y `rove marginal` en las configuraciones de reloj estricto Mkv y Mk de la matriz suministrada, con 32 potencias, 1.000.000 de pasos por potencia y tres semillas por programa. Cada par Rove–BEAST de estimaciones por stepping stone concordó dentro de 2,6 errores estándar combinados, con errores estándar entre 0,20 y 0,35 unidades de log, y el path sampling concordó del mismo modo; una sola implementación recalculó las estimaciones de ambos programas a partir de sus logs. Esto establece concordancia entre implementaciones del mismo estimador en una matriz pequeña, no rendimiento de selección de modelos en datos empíricos.
