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

## Inspeccionar desde el terminal

```sh
target/release/rove inspect fixtures/reference/20x50/matrix.nex
```

El comando emite JSON versionado con recuentos, una vista limitada de celdas,
el hash de entrada y errores separados de compatibilidad con el muestreador
y Mkv. Añade `--gaps missing` solo cuando esa interpretación sea adecuada.
Leer una matriz no escribe resultados del análisis.
