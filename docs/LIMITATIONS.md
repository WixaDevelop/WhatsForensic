# Limitaciones de la herramienta

Este documento enumera explícitamente qué la herramienta **no hace**, para evitar que usuarios o lectores del reporte le atribuyan capacidades que no tiene.

-----

## Lo que la herramienta no hace

### Análisis de evidencia cifrada

- No descifra backups de WhatsApp Android (`msgstore.db.crypt12`, `crypt14`, `crypt15`).
- No descifra backups cifrados de iOS.
- No procesa contenedores Keychain o equivalentes.
- Solo procesa bases SQLite que ya están descifradas y disponibles como archivo plano.

### Recuperación profunda de datos eliminados

- No realiza carving del freelist de SQLite. Las páginas marcadas como libres pueden contener datos recuperables, pero la herramienta en su versión actual no las inspecciona.
- No reconstruye frames antiguos del WAL que hayan sido sobrescritos por checkpoints posteriores.
- No analiza unallocated space dentro de las páginas.
- No procesa rollback journals existentes en estado intermedio: si detecta un `-journal`, lo copia y reporta su presencia, pero no extrae datos pre-edición de él.

### Análisis de artefactos externos a la base SQLite

- No correlaciona con artefactos del sistema operativo (logs, plist, registros, prefetch).
- No analiza tráfico de red ni metadatos de telefonía.
- No verifica la existencia ni integridad de archivos multimedia referenciados desde la base.
- No procesa archivos de configuración de la aplicación.

### Interpretación semántica

- No interpreta el significado del contenido de los mensajes.
- No evalúa la veracidad de la información contenida.
- No infiere intenciones a partir del texto.
- No realiza análisis lingüístico ni de sentimiento.

### Identificación de personas

- No identifica personas a partir de números de teléfono, alias o identificadores. Esa correlación queda a cargo del investigador.
- No accede a directorios telefónicos ni a fuentes externas para enriquecer identidades.

### Firmas y certificación

- No genera firmas digitales criptográficas de los reportes.
- El audit log encadenado por hash provee trazabilidad técnica pero no equivale a una firma legalmente reconocida.
- No emite certificados de integridad reconocidos judicialmente.

### Modificación de la evidencia

- No altera el archivo original bajo ninguna circunstancia.
- En modo “incluye WAL” puede modificar la copia working durante el análisis (es comportamiento esperado de SQLite), pero la copia pristine y el archivo original permanecen intactos.

### Análisis en tiempo real

- No se conecta a dispositivos para extraer bases en vivo.
- No monitorea cambios en archivos durante el análisis.
- Trabaja exclusivamente sobre bases extraídas previamente por otros medios.

-----

## Lo que la herramienta tampoco asume

- No asume que el archivo provisto sea íntegro. Lo procesa tal como llega y reporta lo que encuentra.
- No asume la versión de la aplicación. Detecta heurísticamente y solicita confirmación.
- No asume la zona horaria de los timestamps. El investigador la declara al crear el caso.
- No asume que ausencia de evidencia equivalga a evidencia de ausencia: un mensaje que no aparece puede haber sido eliminado, no haber existido nunca, o estar en una parte de la base no analizada.

-----

## Consecuencias prácticas

Estas limitaciones implican que el reporte generado por la herramienta:

1. **No debe ser citado como prueba autónoma** de eliminación, revocación o presencia de información.
1. **Debe complementarse con análisis adicional** cuando se requiera recuperación profunda de datos eliminados.
1. **No reemplaza herramientas forenses especializadas** en carving SQLite (Belkasoft, Cellebrite Physical Analyzer, FQLite, etc.) cuando la investigación requiere recuperación exhaustiva.
1. **Requiere fuentes complementarias** para identificación de personas y correlación con eventos externos.

-----

## Limitaciones que pueden levantarse en fases futuras

La fase 8 (opcional, no planificada) podría agregar:

- Carving del freelist.
- Reconstrucción de WAL frames antiguos.
- Análisis de unallocated space.

Otras limitaciones (descifrado, análisis de artefactos externos, certificación) están fuera del alcance previsto y requerirían un rediseño sustancial del producto.
