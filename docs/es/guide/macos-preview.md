---
description: "Compila y explora la primera interfaz nativa de Rove: importa una matriz, examina sus caracteres y prepara un análisis."
---

# La vista previa para Mac

Rove tiene una primera interfaz nativa para macOS, creada con SwiftUI y
AppKit. Importa matrices, prepara ajustes y ejecuta el motor Rust incluido
con progreso real y cancelación. No requiere BEAST ni Java.

![La interfaz nativa de Rove](/images/macos-preview.png)

## Acceso a la beta y primera visita

La beta nativa comienza con **I have an invitation** y **Join the beta list**.
Usa un correo aprobado para solicitar un código de ocho dígitos, válido durante
diez minutos. Unirse a la lista también requiere verificar el correo, pero solo
registra tu interés. No concede acceso. El equipo del proyecto aprueba las invitaciones.

Después de confirmar el acceso, una carta de Rafael agradece a Thiago J. Silva
antes de la introducción. **Continue** la marca como leída en este Mac; cerrar
sesión o actualizar no vuelve a mostrarla. La carta sigue el idioma de la app.
Quienes ya usan Rove también la ven una vez, sin repetir el recorrido completado.

Después, una introducción de cuatro pasos presenta el espacio
de trabajo, las matrices, los grupos externos y los resultados. Puedes omitirla,
probar el ejemplo o abrir tu matriz; ningún análisis comienza automáticamente.
Ábrela de nuevo en **Help → Take the Rove tour**. **Help → What's new in Rove**
reúne las novedades por versión y las notas anteriores. Aparecen una vez por
actualización documentada cuando no hay operaciones en curso. No se superponen
al primer recorrido. Volver a estas pantallas conserva la matriz y su configuración.

El acceso se guarda en el Llavero de macOS. Rove lo verifica por internet al abrir
y cada seis horas mientras está activo. Tras una verificación correcta, funciona
sin conexión hasta siete días. Una invitación vencida o revocada impide nuevos
análisis y reanudar puntos guardados; una ejecución en curso puede terminar.
Cierra la sesión desde el menú Help.

Cloudflare verifica el acceso y Resend entrega los códigos. El servicio guarda
el correo, estado de invitación, interés confirmado, idioma y horarios de acceso.
Las matrices, árboles y archivos de análisis permanecen en tu Mac. La lista no es
una suscripción a un boletín. Contacta con [Cortex Labs](https://getcortexlabs.com/)
para solicitar la eliminación de la lista. La CLI Rust independiente no requiere
acceso a la beta ni estos servicios de red.


## Compilar y abrir

La app tiene como destino macOS 14 o posterior. Para compilarla, instale Xcode 26.3 o posterior y Rust en un Mac compatible con esa versión de Xcode;
ejecute estos comandos desde la raíz del repositorio:

```sh
scripts/build-macos.sh
open build/macos/Rove.app
```

Se genera una aplicación local con firma ad hoc. No es una versión notarizada
ni un instalador. La interfaz tiene un tema oscuro inspirado en Cortex Labs
y está disponible en inglés y portugués brasileño.

## Idioma y ajustes

Abre **Settings** desde la barra lateral o pulsa **Command-coma**. Puedes seguir
el idioma del Mac o elegir inglés o portugués brasileño. El cambio es inmediato
y conserva la matriz abierta y los ajustes del análisis. Las preferencias se
guardan en este Mac. También puedes reducir las animaciones y la transparencia
de la barra lateral; las preferencias de accesibilidad del sistema tienen
prioridad. Los diálogos del sistema siguen el idioma del Mac y los diagnósticos
originales del motor permanecen disponibles en **Technical details**.

**Rove → About Rove** muestra el origen del proyecto, la versión de la app y
los créditos de Rafael Augusto (software y diseño) y Thiago J. Silva (biología
e investigación). La ventana sigue el idioma de la app y enlaza al sitio web,
la guía y el repositorio.

## Explorar una matriz

Abre un NEXUS con **Command-O**, arrastra un archivo a la pantalla inicial o
elige **Try an example**. El ejemplo es la matriz de referencia suministrada,
con 20 terminales y 50 caracteres.

La vista muestra taxones, caracteres observados, patrones comprimidos y celdas
ausentes. Haz clic en una celda para ver su taxón, carácter y observación.
Las estadísticas abarcan toda la matriz; la vista desplazable muestra hasta
200 taxones y 120 caracteres. La aplicación admite archivos de hasta 32 MB.

Se pueden analizar matrices binarias y multiestado. **State partitions** agrupa
los caracteres por estados observados/posibles o usa explícitamente el alfabeto
declarado. La matriz y la revisión muestran los recuentos por partición.
Las entradas incompatibles muestran una explicación. Tratar los gaps como
datos ausentes requiere una decisión explícita; los caracteres potencialmente
constantes no pueden usar Mkv.

## Preparar y ejecutar un análisis

Elige el reloj estricto o la [configuración de referencia con reloj relajado](./reference-model),
Mk, Mkv o [Mkparsinf](./character-selection), longitud de cadena, intervalo de muestreo y semilla. Los priors conservan
los valores del motor. En Model, **Add group** selecciona los terminales que deben
formar un clado. Los grupos deben estar anidados o separados. No se restringe
nada automáticamente, tampoco el grupo interno del ejemplo. El soporte impuesto
por una restricción no es recuperación independiente a partir de los datos.

En **Outgroup**, puedes seleccionar un terminal como T20. Todos permanecen en
el análisis: elegir T20 obliga a T1–T19 a formar un clado, con T20 al otro lado
de la división de la raíz. El grupo externo es un terminal, no el ancestro de
la raíz. Este control admite un grupo externo y comienza en **Not selected**.

En **Additional groups**, añade grupos o usa el lápiz para editar uno existente.
Marcar terminales prepara una selección; **Apply group** la confirma y **Cancel**
descarta los cambios pendientes. Los terminales sin marcar siguen en el análisis.
Los conflictos con el grupo externo bloquean la ejecución. Quitar el grupo externo
conserva los grupos manuales; comprueba **Applied groups** en la revisión.

La revisión muestra la división de la raíz, los grupos aplicados y todos los
terminales conservados. También aparecen en **Applied analysis setup** durante
la ejecución y en History. Guardar/reabrir y **Use this setup** conservan la elección;
otra matriz o **New analysis** empieza sin grupo externo. Las configuraciones
antiguas conservan los grupos originales sin inferir un grupo externo. Los nuevos
archivos de formato 2 requieren la aplicación actualizada. Cambiar estas opciones
prepara otro análisis; no cambia la raíz de resultados existentes. El soporte de
un grupo impuesto proviene de la restricción.

![Selección explícita del grupo externo en la aplicación](/images/macos-outgroup.png)

Revisa los ajustes y grupos, elige dónde guardar y pulsa **Start analysis** o
**Command-Return**. El motor Rust incluido se ejecuta localmente mientras la
ventana muestra los pasos realmente completados y el tiempo transcurrido.
Las preferencias siguen disponibles; los ajustes científicos quedan bloqueados
durante la ejecución. No hay estimaciones de finalización, ESS ni soporte inventados.

Cada ejecución tiene una carpeta nueva, por defecto en `~/Rove Runs`. **Open
folder** muestra la configuración guardada, el registro de sesión, los eventos
de progreso y el log del motor. La subcarpeta `results` contiene `trace.log`,
`trees.nexus` y `run.json`. Al terminar, los botones Trace, Trees y Run record
muestran estos archivos en Finder. **Completed** significa que los pasos
solicitados terminaron y se verificaron los metadatos de salida. No demuestra
convergencia. Compara ejecuciones independientes y evalúa los diagnósticos de
muestreo y árboles; consulta la [guía de muestreo](./sampler).

**Pause analysis** guarda el siguiente paso muestreado y detiene el motor. Las cadenas de un grupo pausan juntas. En **History**, selecciona la ejecución y pulsa **Resume analysis**; la tarjeta muestra el paso guardado. Se requieren la misma matriz, configuración y ejecutable de Rove, y la carpeta original.

**Cancel analysis** pide confirmación y conserva archivos parciales. Cerrar la aplicación también detiene el motor. Tras una interrupción, History ofrece recuperación si existe un checkpoint; se repiten los pasos posteriores al último guardado. Las ejecuciones antiguas sin checkpoints no se pueden reanudar. Una configuración guardada prepara una ejecución nueva; reanudar continúa una existente.

## Guardar y reabrir una configuración

**Save setup** escribe un `.rove.json` con la ruta de la matriz original, SHA-256,
ajustes y grupos explícitos. No copia la matriz ni inicia una ejecución.
**Open Setup** (**Command-Shift-O**) restaura la configuración después de verificar
que la matriz original existe y conserva su huella. Se informa de entradas
modificadas o ausentes antes de ejecutar. Revisa los ajustes restaurados antes
de iniciar una nueva ejecución. Una configuración no es un punto de reanudación.

**Copy terminal command** prepara un comando para el motor Rust incluido y una
carpeta nueva en `~/Rove Runs`. Lee la matriz al ejecutarse. El mismo binario,
entrada, ajustes, grupos y semilla producen las mismas muestras científicas
desde la aplicación y el CLI; informar del progreso no modifica el muestreo.

Para automatización, `rove sample ... --events-json` emite líneas JSON versionadas
con eventos `started`, `progress`, `completed`, `cancelled` o `failed`.
`--expect-input-sha256 HASH` rechaza entradas modificadas antes de crear resultados;
`--cancel-on-stdin-close` permite cancelar cuando el proceso propietario cierra
su canal de entrada. La aplicación usa estas opciones. Las [notas del protocolo y proceso](https://github.com/getcortexlabs/rove/blob/main/apps/macos/README.md#cli-event-protocol-version-1)
definen los campos, la validación y los códigos de salida.

## Consultar ejecuciones anteriores

![Historial de Rove con ejecuciones completadas, canceladas y fallidas](/images/macos-history.png)

Abre **History** en la barra lateral o pulsa **Command-Shift-H**. Las ejecuciones
aparecen de la más reciente a la más antigua, con matriz, fecha, estado registrado,
tiempo transcurrido, modelo y semilla. Busca por matriz o semilla, o filtra por
estado. Selecciona una ejecución para ver pasos completados y solicitados,
ajustes, grupos explícitos y la huella de entrada original. Los botones de
archivos muestran las salidas disponibles en Finder.

Las ejecuciones anteriores de la aplicación en `~/Rove Runs` se descubren
automáticamente. Otras ubicaciones de salida elegidas en la aplicación se
recuerdan en este Mac. **Add run folder** añade una ejecución guardada en otro
lugar; elige la carpeta que contiene `session.json` o `setup.rove.json`.
**Refresh** vuelve a comprobar los archivos. La aplicación lee metadatos pequeños,
no el contenido de los trazados ni de los árboles, fuera del hilo de la interfaz.

Se identifican carpetas ausentes, registros ilegibles y salidas incompletas.
Una ejecución sin final confirmado aparece como **Interrupted**, aunque existan
archivos. Los registros antiguos siguen siendo legibles; los valores no disponibles
se muestran con una raya, sin estimaciones. Los nuevos intentos guardan una sesión
antes de comprobar la entrada para conservar cancelaciones inmediatas y fallos
al iniciar; una huella distinta impide generar resultados científicos.

**Use this setup** comprueba la matriz original y abre los ajustes guardados
para revisión. No inicia un análisis automáticamente ni reanuda una cadena.
Durante una ejecución puedes navegar por History y volver mediante **Current
analysis**; reutilizar ajustes queda deshabilitado hasta que termine o se cancele.
**Back to workspace** (**Command-Shift-R**) conserva la matriz y los ajustes
abiertos. El historial no elimina archivos ni demuestra convergencia.

**Compare with other runs** aparece en una ejecución completada. Lista las
demás ejecuciones completadas de History con la misma matriz, modelo,
restricciones e intervalo de muestreo y semillas que no se solapan, incluidos
los grupos. Elige las ejecuciones y un burn-in, y la app ejecuta `rove compare`
sobre las carpetas de cada cadena. El informe es el que se muestra para un grupo
de cadenas, con una columna por cadena, y se guarda en la carpeta `comparisons`
de la ejecución. La concordancia entre ejecuciones no es convergencia de ninguna.

## Inspeccionar desde el terminal

```sh
target/release/rove inspect fixtures/reference/20x50/matrix.nex
```

El comando emite JSON versionado con recuentos, una vista limitada de celdas,
el hash de entrada y errores separados de compatibilidad con el muestreador
y Mkv. Añade `--gaps missing` solo cuando esa interpretación sea adecuada.
Leer una matriz no escribe resultados del análisis.

## Resumir los árboles posteriores

![Resumen de árboles en Rove](/images/macos-summary.png)

Tras completar una ejecución, elige **Summarize trees** en el resultado o en
History. Define el porcentaje de burn-in y las alturas, y pulsa **Create tree
summary**. Los resúmenes guardados reaparecen al abrir el panel. El cálculo
ocurre fuera del hilo de la interfaz y puede cancelarse.

El burn-in descarta los primeros `floor(total de muestras × porcentaje / 100)`
árboles, redondeando hacia abajo. `STATE_0` cuenta como muestra. El 10% inicial
es una sugerencia editable, no un diagnóstico de convergencia; decide después
de revisar el trace. Se muestran las cantidades descartada y retenida. El
burn-in también puede indicarse como número de muestras, como con
`--burnin-samples`. La app no resume ejecuciones parciales, canceladas o
interrumpidas.

MCC (máxima credibilidad de clados) selecciona una **topología muestreada**, no
un consenso mayoritario. Rove suma los logaritmos de las frecuencias de los
clados internos de cada árbol retenido. Ordena los clados antes de sumar; el
primer candidato retenido gana los empates exactos. `posterior` es la fracción
de árboles retenidos que contienen un clado. Los grupos impuestos por
restricciones registradas llevan `constrained=true`: ese soporte no es una
recuperación independiente a partir de los datos.

**Keep selected tree** conserva las alturas de la muestra elegida. **Common
ancestor mean** usa la altura media del ancestro común más reciente del grupo
en **todos** los árboles retenidos. Las ramas son diferencias de altura entre
padre e hijo, en las unidades temporales originales. Las tasas son
multiplicadores separados; no se infieren fechas de calendario ni distancias
escaladas por tasas. Las medias y rangos de altura/tasa son condicionales a
la presencia del clado; `CAheight_mean` y `CAheight_range` usan todos los árboles
retenidos. Los rangos son mínimos y máximos, **no intervalos HPD ni de
credibilidad**. Las medianas de altura y los intervalos HPD no están implementados.

Cada resumen crea una carpeta nueva en `summaries` dentro de la ejecución:

- `mcc.tree`: un árbol NEXUS enraizado y anotado para FigTree. Elige `posterior`
  en Node Labels de FigTree para ver los soportes.
- `samples.trees`: copia exacta de todas las muestras originales, incluido el
  burn-in, para herramientas que esperan la extensión `.trees`.
- `summary.json`: hashes de entrada y ejecutable, metadatos de la ejecución,
  burn-in, cantidad retenida, estado elegido, método, alturas y versión de Rove.

Los botones muestran los archivos en Finder. `results/trees.nexus` y las demás
salidas originales se conservan. Cada resumen requiere espacio para otra copia
del archivo original. Un resumen y un soporte alto no demuestran convergencia;
compara ejecuciones independientes y diagnósticos de muestreo.

El comando Rust independiente hace el mismo cálculo sin Java ni BEAST:

```sh
target/release/rove summarize /path/to/results/trees.nexus \
  --output /path/to/new-summary --burnin-percent 25 --heights ca
```

Usa `--burnin-samples N` para contar muestras o `--heights keep` para conservar
alturas. La carpeta de salida no debe existir. La extensión no cambia la
interpretación. Sin un `run.json` junto a la entrada, la finalización se registra
como **unverified**: un archivo sintácticamente completo no prueba que la
ejecución haya terminado. Con metadatos, se verifican finalización, cantidad e
intervalos de todas las muestras.

Se admiten árboles temporales de Rove enraizados y binarios, con puntas de edad cero
o alturas fósiles explícitas, tabla de traducción e identificadores `STATE_` crecientes.
Se rechazan taxones mezclados, ausencia de `End;`, anotaciones de tasas parciales
y formas no compatibles. Consulte [datación de fósiles](./tip-dating). No se admiten politomías
ni bloques NEXUS externos arbitrarios. Límites: 4.096 taxones, 512 niveles,
2 MiB por instrucción y un millón de clados distintos o una estimación de
128 MiB para sus claves, lo que se alcance primero. Esta estimación no es un
límite rígido de memoria del proceso. Cuatro pasadas secuenciales conservan
conteos y estadísticas del árbol elegido, no todos los árboles en memoria.

[Validación y mediciones en Apple silicon](https://github.com/getcortexlabs/rove/blob/main/benchmarks/tree-summary/2026-10-29/README.md)
registran comparaciones con versiones fijas de TreeAnnotator y lectores NEXUS externos.

## Explorar un árbol anotado

![Explorador nativo del árbol MCC](/images/macos-tree.png)

En un resultado terminado o en History, abre **Summarize trees** y elige
**View tree** en un resumen guardado. Una ventana nativa independiente abre el
resultado MCC; no recorre muestras sin procesar ni presenta `STATE_0` como
resultado final. La cabecera identifica la ejecución y los recuentos retenidos
y descartados. **About this summary** muestra burn-in, política de alturas,
muestra de origen y grupos impuestos.

Selecciona un nodo o una punta para consultar soporte, longitud de rama, alturas
e intervalos de tasas disponibles. **Descendant taxa** permite navegar por los
hijos o seleccionar una punta. Las anotaciones ausentes aparecen como
**Unavailable**. Los rombos indican grupos impuestos; su soporte no constituye
evidencia independiente. Un grupo externo verificado se identifica como punta,
no como nodo raíz. El orden vertical de las puntas no determina el enraizamiento.

Desplázate o arrastra para mover, usa el gesto de pinza o los botones de zoom y
**Fit width** para volver al 100%. Los árboles altos siguen siendo desplazables
para mantener legibles las etiquetas. La búsqueda encuentra taxones en clados
contraídos y expande sus ancestros al seleccionarlos. **Rotate**, **Collapse
clade** y **Reset display** cambian solo el dibujo. Con el lienzo enfocado,
Arriba/Abajo recorren nodos visibles, Izquierda selecciona el padre, Derecha un
hijo y Espacio contrae o expande. El inspector ofrece también botones nativos
de navegación. Señalar un nodo o una punta resalta su camino hasta la raíz
sin cambiar la selección. Estas operaciones no tienen animación.

**Support** controla las etiquetas; se omiten algunas en ramas cortas o filas
cercanas para evitar superposiciones, pero sus valores siguen en el inspector.
Los nombres largos se abrevian en el dibujo y aparecen completos en la búsqueda
y el inspector. La escala y las ramas usan unidades de tiempo del origen.
Las tasas son multiplicadores separados; los rangos observados no son HPD.

**Export SVG** guarda la disposición completa, incluidas las rotaciones y los
clados contraídos, con escala, leyenda y procedencia. El zoom y el desplazamiento
seleccionan la zona visible, no un recorte de la figura exportada. SVG es una
imagen vectorial; `mcc.tree`, las muestras originales y los archivos de ejecución
permanecen intactos. Este visor no incluye PNG, importación de árboles externos
ni reenraizamiento.

El comando incluido `rove inspect-tree /ruta/al/resumen` valida el registro y
la huella del árbol antes de emitir JSON versionado para la visualización.
Lectura, disposición, búsqueda y generación de SVG se ejecutan fuera del hilo
de interfaz. El visor admite un árbol MCC de Rove de hasta 16 MiB, 4.096 taxones
y 512 niveles de anidamiento; los límites anteriores del lector de muestras
no cambian. Un resumen sigue sin demostrar convergencia.

## Inspeccionar los diagnósticos

![Diagnósticos de la traza con un 25% de burn-in en la aplicación nativa](/images/macos-diagnostics.png)

Elige **Diagnostics** en un resultado finalizado o en una entrada de History
con archivo de muestras. La ventana enumera todos los parámetros, empezando por
posterior, likelihood y prior. Selecciona una fila para ver media, desviación
estándar, ESS, intervalo del 95% de colas iguales y gráfico. `posterior` es una
**densidad logarítmica**, no el soporte posterior de un nodo.

Cambia **Burn-in (%)** y pulsa **Apply**. Los recuentos y el primer estado retenido
corresponden al cálculo aplicado; una edición pendiente no cambia las etiquetas
del resultado anterior. La regla coincide con el resumen de árboles: descartar
`floor(N × porcentaje / 100)` muestras, incluyendo `STATE_0` en una ejecución
estándar. **Summarize trees with this burn-in** abre el panel con ese porcentaje.
Ambos lectores contrastan estados e intervalos con el registro de ejecución.

**Whole run** incluye el inicio descartado en gris y marca el límite del burn-in.
**After burn-in** ajusta el gráfico a los puntos retenidos sin cambiar los cálculos.
El control bajo el gráfico permite consultar estados y valores con el teclado.
Tabla, búsqueda y controles ofrecen navegación nativa; el gráfico no tiene
transiciones animadas. Se muestran hasta 1.028 puntos por parámetro, conservando
primero, mínimo, máximo y último de cada bloque, además del límite del burn-in.
**Los cálculos usan todas las muestras retenidas**, nunca solo los puntos dibujados.

ESS estima información equivalente a muestras independientes, no pasos de MCMC.
Usamos la convención de autocovarianzas positivas por pares de BEAST 2.7.8, con
retardos inferiores a 2.000; no es ESS de centro o colas normalizado por rangos.
Las series constantes o con menos de cuatro valores retenidos tienen ESS no
disponible. Se señala si la correlación positiva alcanza el límite de retardos
evaluado. En esta vista de una sola cadena, estos números no activan un sello de convergencia ni una parada automática. La parada opcional entre varias cadenas se configura en el plan de exploración.

El intervalo usa los percentiles 2,5 y 97,5 con interpolación lineal (tipo 7 de R).
Describe la distribución retenida; **no es HPD** ni el error de Monte Carlo de la
media. Una única muestra no tiene intervalo y las series muy cortas se señalan.
Mezcla escalar, acuerdo entre ejecuciones independientes y convergencia topológica
son cuestiones distintas: esta vista no calcula R-hat ni compara clados.

Cada cálculo correcto guarda `diagnostics/trace-*/diagnostics.json` en una carpeta
nueva, con huella del origen, burn-in, estados, métodos, estadísticas y puntos.
**Calculation record** muestra el archivo en Finder. Al volver a abrir, se recuerda
el último porcentaje correcto y se recalcula el archivo actual. Los informes
anteriores y archivos originales permanecen intactos. Lectura y cálculo se ejecutan
fuera del hilo de interfaz y pueden cancelarse mientras otras cadenas continúan.

Las ejecuciones parciales o no verificadas se identifican. Se rechazan valores no
finitos, columnas inconsistentes, estados irregulares y líneas finales incompletas.
También se rechaza un registro terminado al que le falten muestras. Sin `run.json`,
un archivo válido no demuestra finalización. Los límites son 128 MiB, un millón
de filas, 128 parámetros y ocho millones de celdas numéricas; no limitan toda
la memoria del proceso.

El comando independiente usa el mismo cálculo Rust:

```sh
target/release/rove diagnose /ruta/results/trace.log --burnin-percent 25 \
  --output /ruta/nuevo-diagnostico
```

Omite `--output` para emitir JSON o usa `--burnin-samples N` en lugar del porcentaje.
La carpeta de salida debe ser nueva. No se requiere Java ni Tracer.
[Validación y mediciones](https://github.com/getcortexlabs/rove/blob/main/benchmarks/diagnostics/2026-10-29/README.md)
incluyen series sintéticas y una referencia BEAST con versión fijada.

## Comparar cadenas independientes

![Comparación de cuatro cadenas independientes cortas](/images/macos-chains.png)

En **Model → The exploration plan**, elige **1, 2 o 4 cadenas**. El número de pasos se aplica a cada cadena. Varias cadenas se ejecutan a la vez con semillas distintas y registradas; el progreso muestra cada una por separado. Cancelar cierra todo el grupo.

Los grupos completados y sus detalles en History incluyen **Do the chains agree?**, con R-hat y ESS para todas las estadísticas registradas y una comparación de frecuencias de clados. Selecciona una cadena para abrir sus diagnósticos, resumir sus árboles o ver su carpeta. Las configuraciones guardadas conservan el número de cadenas; las antiguas siguen usando una sola. Los archivos permanecen separados para no tratar el burn-in de una cadena como continuación de otra.

**Stop when diagnostic thresholds are met** es opcional y está desactivado por defecto. Usa 10% de burn-in, ESS ≥ 200 por cadena, R-hat ≤ 1,01 en todas las estadísticas escalares registradas, al menos 1.000 muestras retenidas por cadena y dos comprobaciones satisfactorias consecutivas. Se guardan el paso y el motivo de parada. Son criterios escalares, no una garantía de convergencia topológica; revisa clados y traces. Consulta [cadenas independientes](./sampler#cadenas-independientes) para las opciones y límites de la CLI.

**Advanced sampling**, plegado bajo el plan de exploración, expone las demás opciones de `rove sample`. **Sample from the priors only** se ejecuta sin los caracteres. Con la parada automática activada, el ESS mínimo, el R-hat máximo y el intervalo de comprobación pueden sustituir los valores por defecto del motor, 200, 1,01 y su propia cadencia de comprobación. El intervalo de checkpoint puede darse en pasos en lugar del valor por defecto de unos 30 segundos. El esquema de propuestas ofrece el clásico por defecto, los dos esquemas evaluados en los informes del [reloj relajado](https://github.com/getcortexlabs/rove/blob/main/benchmarks/sampling/2026-10-29/README.md) y de la [frecuencia de la gamma](https://github.com/getcortexlabs/rove/blob/main/benchmarks/gamma-mixing/2026-10-30/README.md), y pesos y tamaños de paso iniciales personalizados por movimiento, con un ajuste de paso que puede desactivarse. Cada opción se valida antes de iniciar el motor, se registra en el registro de la ejecución y en History, y se guarda en las configuraciones como formato 5, que las versiones anteriores de la app rechazan en lugar de ejecutar sin ella. Ninguno de los esquemas evaluados se recomienda; los informes registran lo que cada uno ganó y perdió.

## Probar este Mac

[Probar este Mac](./mac-benchmark)
