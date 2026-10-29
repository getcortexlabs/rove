---
description: "Compila y explora la primera interfaz nativa de Rove: importa una matriz, examina sus caracteres y prepara un análisis."
---

# La vista previa para Mac

Rove tiene una primera interfaz nativa para macOS, creada con SwiftUI y
AppKit. Importa matrices mediante el motor Rust y prepara los ajustes del
análisis. La ejecución y el seguimiento del progreso dentro de la ventana
todavía no están implementados.

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

## Preparar un análisis

Elige el reloj estricto o la [configuración de referencia con reloj relajado](./reference-model),
Mk o Mkv, longitud de cadena, intervalo de muestreo y semilla. Revisa las
opciones antes de guardar. Los priors mantienen los valores del motor y no
se restringe ningún grupo automáticamente. La longitud de la cadena por sí
sola no establece convergencia.

**Save setup** guarda un `.rove.json` con la ruta de la matriz original,
SHA-256 y ajustes. No copia la matriz ni inicia una ejecución. Reabrir estos
registros en la aplicación todavía no está implementado.

**Copy terminal command** prepara un comando para el motor Rust incluido.
Pegarlo en Terminal inicia un análisis real usando la matriz original en el
momento de la ejecución. El comando crea una carpeta nueva bajo `~/Rove Runs`.
La [guía de muestreo](./sampler) explica las salidas y los diagnósticos.

## Inspeccionar desde el terminal

```sh
target/release/rove inspect fixtures/reference/20x50/matrix.nex
```

El comando emite JSON versionado con recuentos, una vista limitada de celdas,
el hash de entrada y errores separados de compatibilidad con el muestreador
y Mkv. Añade `--gaps missing` solo cuando esa interpretación sea adecuada.
Leer una matriz no escribe resultados del análisis.
