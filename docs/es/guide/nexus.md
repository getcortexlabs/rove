---
description: "Leer matrices NEXUS morfológicas en Rust, preservar la semántica de las observaciones y comprimir columnas repetidas dentro de particiones de modelo explícitas."
---

# Lectura de NEXUS morfológico

`nexus::parse` lee una cadena UTF-8 en una `data::Matrix`. Preserva el orden de los taxones, el alfabeto de estados declarado, las etiquetas de caracteres y de estados, y cada celda observada. No tiene dependencias en tiempo de ejecución y no invoca a BEAST.

Prueba la matriz de referencia incluida desde la raíz del repositorio:

```sh
cargo run --release --example read_nexus -- fixtures/reference/20x50/matrix.nex
```

```text
20 taxa, 50 characters, 2 declared states
23 observed patterns (50 total weight)
```

Este ejemplo trata todos los caracteres como una sola partición y rechaza gaps (huecos) y polimorfismo al preparar las entradas de verosimilitud. Importa datos; no ejecuta inferencia.

## Sintaxis admitida

Una entrada inválida o no admitida devuelve `nexus::Error` con línea y columna contadas desde 1 y una explicación.

| Elemento | Soporte |
| --- | --- |
| Bloques | Un bloque `DATA` o `CHARACTERS`, opcionalmente precedido por un bloque `TAXA`. Los bloques `TREES` se omiten; los árboles no se importan. Los demás bloques se rechazan. |
| Dimensiones | `NTAX` y `NCHAR` positivos. Un bloque `TAXA` previo puede suministrar `NTAX`. `NEWTAXA` requiere `NTAX`. Todos los taxones declarados deben aparecer exactamente una vez en la matriz. |
| Formato | `DATATYPE=STANDARD`, `SYMBOLS`, `MISSING`, `GAP`, `RESPECTCASE`, `LABELS`, `NOTOKENS`, e `INTERLEAVE` o `INTERLEAVE=YES/NO`. |
| Alfabeto | Al menos dos símbolos ASCII válidos, un carácter por estado. Por defecto, `01`. El orden declarado se preserva, incluidos los estados no observados. |
| Filas secuenciales | Filas con nombre, con espacios en blanco opcionales entre celdas y saltos de línea dentro de la fila. |
| Filas intercaladas | Cada página repite todas las etiquetas de taxones en el mismo orden. Cada fila ocupa una línea, con el mismo número de caracteres dentro de una página. El ancho de las páginas puede variar. Las líneas en blanco son opcionales. No se admiten páginas de continuación sin etiquetas. |
| Nombres | Nombres entre comillas simples, apóstrofos duplicados y nombres sin comillas. Los guiones bajos se convierten en espacios. La comparación y la detección de duplicados ignoran la distinción entre mayúsculas y minúsculas ASCII; los nombres devueltos conservan su escritura. |
| Etiquetas | `CHARLABELS` con exactamente `NCHAR` nombres; `STATELABELS` y `CHARSTATELABELS` indexados. Las etiquetas descriptivas de estados no reducen el alfabeto declarado. |
| Comentarios | Comentarios anidados entre corchetes, incluidos comentarios dentro de palabras. Se aceptan finales de línea LF, CRLF y CR. |

Con `TAXLABELS`, las filas devueltas siguen el orden declarado aunque las filas de la matriz aparezcan en otro orden. En caso contrario, la primera página de la matriz define el orden.

Los valores por defecto son estados sin distinción de mayúsculas y minúsculas, símbolo de faltante `?`, ningún símbolo de gap y filas secuenciales. Por tanto, `-` no es un gap salvo que se declare con `GAP=-`. Los símbolos de estado, de faltante y de gap deben ser distintos bajo la política de mayúsculas elegida.

Los tipos de datos moleculares o continuos, `TOKENS`, `TRANSPOSE`, `NOLABELS`, `EQUATE`, `MATCHCHAR`, los rangos de estados, las frecuencias/recuentos, `ELIMINATE`, las matrices múltiples y los comandos no admitidos se rechazan. `SETS`, `ASSUMPTIONS` y los bloques específicos de aplicaciones también se rechazan: sus particiones, pesos, orden o exclusiones no deben desaparecer en la importación. El análisis sintáctico de XML queda fuera de esta API.

## Observaciones y verosimilitudes de los terminales

La distinción entre llaves y paréntesis sigue la [especificación de NEXUS](https://doi.org/10.1093/sysbio/46.4.590). Para un alfabeto declarado `012`:

| Entrada | Celda almacenada | Vector de verosimilitud del terminal |
| --- | --- | --- |
| `1` | `Cell::State(1)` | `[0, 1, 0]` |
| `{01}` | `Cell::Uncertain([0, 1])` | `[1, 1, 0]` |
| `(01)` | `Cell::Polymorphic([0, 1])` | Rechazada en la conversión a verosimilitud |
| `?` | `Cell::Missing` | `[1, 1, 1]` |
| Gap declarado | `Cell::Gap` | Rechazada, o `[1, 1, 1]` con `GapTreatment::Missing` explícito |

La incertidumbre significa uno de los estados listados; el polimorfismo registra varios estados presentes. El modelo de observación Mk actual no representa el polimorfismo. La importación lo conserva para su inspección, pero la compresión devuelve un error que identifica el taxón y el carácter. No hay conversión automática a incertidumbre. Los gaps también requieren una elección explícita, porque los gaps morfológicos pueden representar inaplicabilidad.

Los vectores de incertidumbre son máscaras de verosimilitud, no distribuciones de probabilidad: `{01}` aporta uno por cada estado permitido, sin dividir entre dos. Los conjuntos de estados se ordenan y deduplican al importar.

## Compresión de columnas repetidas

`Matrix::compress` conserva su comportamiento original: rangos contiguos, no vacíos y contados desde cero que cubren la matriz una vez en orden; cada partición usa el alfabeto declarado completo. La partición automática y los conjuntos explícitos se describen abajo.

```rust
use rove::{data::GapTreatment, nexus};

let input = std::fs::read_to_string("fixtures/reference/20x50/matrix.nex")?;
let data = nexus::parse(&input)?;
let sites = 0..data.character_count();
let partitions = data.compress(std::slice::from_ref(&sites), GapTreatment::Reject)?;
let patterns = &partitions[0];

assert_eq!(patterns.weights().len(), 23);
assert_eq!(patterns.weights().iter().sum::<usize>(), 50);
```

`Patterns::weights` contiene multiplicidades enteras exactas. `site_to_pattern` asocia cada carácter original dentro de la partición a un patrón, numerado por primera aparición. `tip_partials` contiene valores dispuestos por patrones para cada taxón, listos para `LikelihoodCore::set_tip_partials`. Convierte los pesos enteros a `f64` al llamar a `log_likelihood` del núcleo.

La compresión compara las observaciones completas de las celdas, incluido su tipo. Las celdas faltantes e inciertas siguen siendo patrones separados aunque sus máscaras de verosimilitud coincidan. Las columnas idénticas entre particiones también siguen separadas. Esto preserva las observaciones originales a través de la matriz y la correspondencia, al costo de conservar ocasionalmente cálculos redundantes.

Solo se comprimen las columnas observadas. Las columnas constantes y las que solo tienen datos faltantes permanecen, y no se añaden patrones artificiales de ascertainment. La [corrección Mkv](./kernel#condicionar-a-caracteres-variables) añade sus patrones constantes por separado y rechaza los caracteres que podrían ser constantes.

Las pruebas leen ambos [fixtures de 20 × 50](./fixtures), reconstruyen sus columnas a partir de la correspondencia de patrones y comparan verosimilitudes comprimidas y sin comprimir sobre matrices generadas con datos inciertos y faltantes.

## Particiones por número de estados

`Matrix::partition_by_states(gaps)` agrupa los caracteres por el número de estados distintos en sus observaciones. Los conjuntos explícitos de incertidumbre aportan sus estados posibles; los datos ausentes y gaps no añaden estados. Un carácter con `0`, `2` y `3` tiene tres estados. Cada carácter se recodifica en un alfabeto compacto siguiendo el orden declarado. El modelo Mk es no ordenado; las etiquetas numéricas no exigen cambios ordenados.

Un alfabeto declarado de dos estados conserva ambos, incluso en caracteres constantes o totalmente ausentes. Con un alfabeto mayor, una columna con menos de dos estados observados/posibles no determina su modelo y se rechaza en la partición automática. Usa `CharacterSet { sites, symbols }` con `Matrix::compress_sets`, o `--partitioning declared` si todos los estados de la matriz son posibles para cada carácter. `CHARSTATELABELS` sigue siendo descriptivo y no define estos alfabetos. Los bloques NEXUS `SETS` siguen rechazándose.

`compress_sets` admite índices ordenados, no necesariamente contiguos, contados desde cero. Los conjuntos deben cubrir cada carácter exactamente una vez e incluir todos los estados observados/posibles. `Patterns::sites()` enumera las posiciones originales; `state_symbols()` registra las etiquetas originales de cada carácter en el orden del modelo. La compresión se realiza tras recodificar, dentro de cada partición, conservando el peso total.

`rove inspect` y `rove sample` usan `--partitioning observed` por defecto; ambos aceptan `--partitioning declared`. La app permite elegirlo en **State partitions** y muestra el número de caracteres por partición. Los gaps siguen requiriendo una decisión explícita. Mkv rechaza cualquier carácter que pueda ser constante, aunque se conozca su número de estados.

Cada partición tiene sus propios patrones constantes y corrección Mkv sobre el árbol, reloj y categorías gamma compartidos. `run.json` registra modo, estados, índices contados desde uno, correspondencias y patrones observados. Los checkpoints conservan las cachés de todas las particiones. Doce casos de estado fijo, con tres escalas de longitud de rama, concuerdan con BEAST 2.7.8 / MM 1.2.1 con diferencia absoluta máxima de log-verosimilitud de `1,34e-9`; consulta el [caso de validación](https://github.com/getcortexlabs/rove/tree/main/fixtures/validation/multistate).
