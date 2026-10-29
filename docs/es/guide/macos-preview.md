---
description: "Compila y explora la primera interfaz nativa de Rove: importa una matriz, examina sus caracteres y prepara un análisis."
---

# La vista previa para Mac

Rove tiene una primera interfaz nativa para macOS, creada con SwiftUI y
AppKit. Importa matrices, prepara ajustes y ejecuta el motor Rust incluido
con progreso real y cancelación. No requiere BEAST ni Java.

![La interfaz nativa de Rove](/images/macos-preview.png)

## Compilar y abrir

En macOS 14 o posterior, con Xcode compatible con Swift 6 y Rust instalado,
ejecuta estos comandos desde la raíz del repositorio:

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

## Explorar una matriz

Abre un NEXUS con **Command-O**, arrastra un archivo a la pantalla inicial o
elige **Try an example**. El ejemplo es la matriz de referencia suministrada,
con 20 terminales y 50 caracteres.

La vista muestra taxones, caracteres observados, patrones comprimidos y celdas
ausentes. Haz clic en una celda para ver su taxón, carácter y observación.
Las estadísticas abarcan toda la matriz; la vista desplazable muestra hasta
200 taxones y 120 caracteres. La aplicación admite archivos de hasta 32 MB.

Poder importar no implica poder analizar. Una matriz multiestado puede
visualizarse, pero el muestreador actual solo acepta caracteres binarios.
Las entradas incompatibles muestran una explicación. Tratar los gaps como
datos ausentes requiere una decisión explícita; los caracteres potencialmente
constantes no pueden usar Mkv.

## Preparar y ejecutar un análisis

Elige el reloj estricto o la [configuración de referencia con reloj relajado](./reference-model),
Mk o Mkv, longitud de cadena, intervalo de muestreo y semilla. Los priors conservan
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

**Cancel analysis** pide confirmación antes de detenerse. Los archivos parciales
quedan identificados; no son un análisis completo ni un punto de reanudación.
Cerrar la ventana principal o salir durante un análisis pide confirmación y
espera a que termine el motor. Una parada forzada puede dejar archivos
incompletos. Todavía no es posible reanudar una cadena.

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

## Inspeccionar desde el terminal

```sh
target/release/rove inspect fixtures/reference/20x50/matrix.nex
```

El comando emite JSON versionado con recuentos, una vista limitada de celdas,
el hash de entrada y errores separados de compatibilidad con el muestreador
y Mkv. Añade `--gaps missing` solo cuando esa interpretación sea adecuada.
Leer una matriz no escribe resultados del análisis.
