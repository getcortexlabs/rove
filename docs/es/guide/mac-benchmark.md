---
description: "Mide el motor incluido en Rove en tu Mac y comparte el resultado si quieres."
---

# Probar este Mac

Abre **Benchmark this Mac** en la barra lateral o el menú Help y elige **Run benchmark**. Rove usa su motor y una matriz sintética incluidos en la aplicación. No necesita BEAST, Java, herramientas de desarrollo ni internet para la prueba. El acceso beta mantiene sus reglas normales de uso sin conexión.

Para comparar mejor, conecta la alimentación, desactiva el modo de bajo consumo y cierra otras aplicaciones exigentes. Una prueba no puede empezar mientras hay un análisis activo; tampoco puedes iniciar o reanudar un análisis durante la prueba. Puedes cancelarla en cualquier momento. Cerrar Rove también detiene su motor.

## Qué se mide

El protocolo fijo `rove-mac-v1` utiliza 90 taxones y 250 caracteres binarios simulados bajo Mk, incluidos los constantes. El modelo tiene cuatro categorías gamma, reloj lognormal relajado y ningún filtro de ascertainment. Cada observación empieza desde cero con las mismas semillas: 5.000 propuestas de calentamiento sin cronometrar por cadena, seguidas de 50.000 propuestas medidas por cadena. El ajuste de operadores sigue activado.

Hay tres observaciones para **una cadena** y tres para **cuatro cadenas paralelas**. El segundo caso realiza cuatro veces más trabajo. Las tarjetas muestran tiempo mediano, propuestas totales por segundo y los tres tiempos individuales. La lectura, preparación, calentamiento y escritura de resultados científicos quedan fuera del intervalo medido. El intervalo paralelo incluye el coste de planificar y esperar los hilos.

Mide una carga fija, no convergencia, calidad científica ni duración de tu análisis. No compara con BEAST. Compara el mismo protocolo y huella del motor, separando las cantidades de cadenas. Energía, temperatura, tareas en segundo plano, RAM y núcleos pueden cambiar los tiempos incluso con el mismo chip.

## Resultados y envío

Cada prueba tiene un registro en `~/Library/Application Support/Rove/Benchmarks`. **Previous tests** abre resultados anteriores; **Saved results** muestra la carpeta. Las pruebas canceladas, interrumpidas o inválidas no se pueden enviar. No hay envíos automáticos.

**Share with Cortex Labs** envía el resultado completado seleccionado al banco Cloudflare D1 del proyecto. Incluye:

- Las seis mediciones, cantidades de cadenas y comprobaciones de los estados finales sintéticos.
- Modelo de Mac, chip, núcleos físicos/lógicos y de rendimiento/eficiencia disponibles, memoria, macOS, arquitectura y estado de Rosetta.
- Alimentación, modo de bajo consumo y categoría térmica antes y después.
- Versiones y huellas de los ejecutables de la aplicación y el motor, backend, protocolo y huella de la entrada.
- Un identificador aleatorio por prueba y su fecha.

El registro no incluye matrices, árboles, traces o rutas de investigación, número de serie ni email del beta. La sesión beta autoriza el envío; el servidor mantiene hashes temporales de cuenta/IP para limitar solicitudes, separados de los resultados. No hay ranking público ni identificador permanente del dispositivo. Son resultados declarados por el cliente, no certificados por hardware.

Si el envío falla, el resultado permanece en el Mac para reintentarlo. Reenviar el mismo registro no lo duplica. Para solicitar su eliminación del banco, proporciona el ID del JSON local. Borrar el archivo local no elimina la copia compartida.

El [protocolo y la entrada sintética](https://github.com/getcortexlabs/rove/tree/main/fixtures/benchmarks) están versionados con el motor. `rove benchmark` también emite progreso y mediciones en líneas JSON sin escribir salidas de análisis.
