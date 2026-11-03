---
description: "Combine morfología y ADN alineado con fuentes, correspondencia de taxones y tasas relativas explícitas."
---

# Morfología y ADN juntos

Elija **Combine morphology and DNA** en la pantalla inicial del Mac. Seleccione
una matriz morfológica NEXUS STANDARD y ADN alineado NEXUS/FASTA, revise la
política de taxones ausentes y guarde una entrada `.mixed.json`. En la CLI:

```sh
rove combine --morphology morphology.nex --dna alignment.fasta --missing-taxa reject --output study.mixed.json
rove sample study.mixed.json --output combined-run --seed 42 --steps 30000000 --sample-every 3000 --model reference --morphology-rate estimate --morphology-rate-log-sd 1
```

El manifiesto incorpora los textos UTF-8 originales y sus hashes SHA-256; no
sigue rutas externas y funciona después de mover las fuentes. No sobrescribe
archivos existentes. Se verifican formato, versión, tipos y hashes al importar.
Contiene datos de investigación: compartirlo requiere permiso para distribuir
ambas fuentes.

Los taxones coinciden por **nombre normalizado exacto**, no por posición de
fila. Se aplica la conversión existente de guiones bajos a espacios. `reject`
exige los mismos taxones. `--missing-taxa missing` conserva su unión y completa
las ausencias con datos faltantes; la app muestra los taxones ausentes por fuente.
Deben compartir al menos dos taxones. Las diferencias solo de mayúsculas se
rechazan; revise otras diferencias de escritura antes de considerarlas ausencias
biológicas. Las celdas faltantes no aportan información de estado; los gaps
mantienen `--gaps reject|missing`. Rove no alinea secuencias.

Cada bloque conserva alfabeto, orden, etiquetas y observaciones. La morfología
admite alfabetos observados, declarados o completos mediante etiquetas, caracteres
ordenados/no ordenados y `--conditioning none|variable|informative`. Estas
opciones afectan **solo a la morfología**. El ADN usa JC69 sin condicionamiento,
las cuatro bases y todos los sitios, incluidos los constantes. La ambigüedad
IUPAC representa bases posibles; el polimorfismo verdadero sigue rechazándose.

Ambos bloques comparten topología, tiempos de ramas, reloj estricto o lognormal
y forma gamma cuando está habilitada. Las categorías se integran por sitio.
No hay relojes ni formas gamma independientes por bloque. El ADN fija la tasa
relativa en 1. Todas las particiones morfológicas usan un multiplicador positivo
común `r`: distancia `tiempo × tasa de rama × r`; el ADN usa `tiempo × tasa de rama`.
Esto fija la escala relativa, sin convertir longitudes no datadas en edades de
calendario. La datación fósil conserva su reloj global y calibraciones explícitas.

Seleccione el modelo de tasa:

- `--morphology-rate 2.5` fija la razón en ese valor positivo.
- `--morphology-rate estimate --morphology-rate-log-sd s` estima con
  `log(r) ~ Normal(0, s²)`. Si se omite, la desviación del log es 1; la mediana
  es 1 y la media `exp(s²/2)`. Evalúe la sensibilidad a esta premisa.

El log prior normalizado es
`−log(r) − log(s) − log(2π)/2 − (log(r)/s)²/2`.
La propuesta usa `f ~ Uniform(a, 1/a)` y log de razón de Hastings `−log(f)`.
`morphology_rate_scale` tiene peso inicial 3 al estimar y no está disponible para
una razón fija. El estado se guarda en `trace.log` y checkpoints. La log
verosimilitud suma ambos bloques, con la corrección morfológica elegida. Los
priors se cuentan una vez y el prior relativo está normalizado para estimación marginal.

Las configuraciones de formato 12 conservan fuentes, política de taxones y tasas.
Revisión, historial, importación CLI, comparaciones, reanudación y estimaciones
marginales verifican esa identidad. La predicción reconstruye la razón muestreada
y genera réplicas `.mixed.json`, con alfabetos y máscaras de datos faltantes por
bloque. Incertidumbre y polimorfismo requieren un modelo de observación para
simular. Incluir la matriz al exportar es explícito, también para las copias
originales guardadas por la predicción. Las simulaciones son sintéticas, no sustitutos
de las observaciones.

La validación incluye 36 estados fijos BEAST.base 2.7.8 + MM 1.2.1, sumas
independientes de particiones, cuadratura del prior y muestreo estacionario,
restricciones, recálculo de caché, reanudación exacta, predicción y ejecución
nativa. La tolerancia 1e-7 incluye aproximaciones independientes de cuantiles
gamma. Datos y generador: `fixtures/validation/mixed` y `tools/beast/MixedStates.java`.
Son controles de corrección, no certificación de convergencia ni mediciones de velocidad.
