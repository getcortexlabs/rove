---
description: "ADN alineado en NEXUS y FASTA bajo JC69, con ambigüedades y tratamiento explícito de gaps."
---

# ADN alineado con JC69

Abra una matriz NEXUS `DATATYPE=DNA` o un FASTA ya alineado. La app selecciona
**DNA · JC69**, mantiene A/C/G/T posibles en cada sitio e incluye sitios
constantes. Rove no alinea, recorta ni ensambla lecturas. `sample`, `inspect`
y `marginal` aceptan el mismo archivo:

```sh
rove sample alignment.fasta --output dna-run --seed 42 --steps 30000000 --sample-every 3000 --model reference --gaps missing
```

Elija `--gaps missing` solo si los gaps no deben aportar información de estado;
por defecto se rechazan. `?` es faltante, `R` permite A/G, `Y` C/T y los otros
códigos IUPAC mantienen sus estados posibles; `N` permite los cuatro. Son
máscaras de verosimilitud con unos, no probabilidades fraccionarias ni estados
adicionales. El polimorfismo entre paréntesis se conserva pero no se analiza.
ADN usa `--conditioning none --partitioning declared`; se rechazan Mkv,
Mkparsinf y la recodificación morfológica solicitados explícitamente.

FASTA admite secuencias en varias líneas de igual longitud positiva. El primer
identificador tras `>`, hasta el espacio, nombra el taxón; se ignora la descripción
según la [convención FASTA](https://www.ncbi.nlm.nih.gov/genbank/fastaformat/).
Se aplica la convención NEXUS existente: guiones bajos pasan a espacios, las
duplicaciones ignoran mayúsculas ASCII y se conserva el resto de la grafía.
Revise los nombres mostrados antes de enraizar o combinar datos. Los originales
no cambian. En NEXUS DNA las bases no distinguen mayúsculas, el alfabeto es
A/C/G/T fijo y el gap predeterminado es `-`; `SYMBOLS="ACGT"` es opcional.
Se rechazan alfabetos propios, RESPECTCASE, ADN ordenado, ARN, proteínas y
directivas no admitidas. `MISSING=N` declara N como faltante. Se admiten los
mismos diseños secuenciales/intercalados que para morfología.

JC69 fija frecuencias 1/4 y tasas de intercambio iguales, normalizadas a una
sustitución esperada por sitio: `Pii(t)=1/4+3/4 exp(−4t/3)` y
`Pij(t)=1/4−1/4 exp(−4t/3)` entre bases diferentes. Los controles existentes
ofrecen reloj estricto o relajado lognormal, categorías gamma, raíz y priors
de árbol admitidos. No se aplica corrección morfológica de selección. HKY,
GTR y análisis mixto quedan fuera de esta entrega.

Configuraciones de formato 11, historial, registros y checkpoints conservan
ADN/JC69 y la política de gaps; apps anteriores rechazan estas configuraciones.
Se pueden importar ejecuciones CLI al historial. Modelos incompatibles no se
combinan para convergencia. Árboles, diagnósticos y estimaciones marginales
conservan el modelo. Pruebas cortas verifican integración, no convergencia.
La predicción requiere bases individuales o faltantes; la incertidumbre IUPAC
necesita un modelo de observación y se rechaza en simulación. Las réplicas se
escriben como NEXUS DNA. La exportación guarda los bytes originales en
`matrix.fasta` o `matrix.nex`. La inclusión es explícita y también controla
las copias originales guardadas por las verificaciones predictivas.

`fixtures/validation/dna` incluye datos sintéticos, suma analítica independiente
de estados internos, seis estados fijos BEAST.base 2.7.8, frecuencias simuladas
y reanudación exacta. Con iguales tasas de categoría, las diferencias absolutas
de log verosimilitud son menores que 1e-10. Para cuantiles gamma independientes
se usa tolerancia 1e-7; la aproximación AS91 de BEAST difiere menos de 3e-8 en
las tasas de estos estados. No es una comparación de velocidad.
