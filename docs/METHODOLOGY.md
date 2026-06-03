# Metodología y limitaciones de análisis

Este documento describe la metodología aplicada por la herramienta. Su contenido se incluye literalmente en la hoja “Metodología y limitaciones” de cada reporte XLSX generado.

-----

## 1. Naturaleza de la herramienta

La herramienta realiza análisis forense de bases SQLite extraídas de dispositivos móviles o backups. **No es una herramienta certificada judicialmente.** Sus resultados son insumos técnicos que deben ser interpretados por un perito o investigador competente, y correlacionados con otras fuentes antes de extraer conclusiones.

La herramienta no reemplaza el juicio del investigador. Las anomalías detectadas son indicios, no pruebas.

-----

## 2. Preservación de la evidencia

Para cada archivo ingresado al caso, la herramienta:

1. Calcula el hash SHA-256 del archivo original antes de cualquier otra operación.
1. Realiza dos copias dentro del workspace del caso:
- **Copia pristine:** copia inmutable que nunca es abierta con SQLite. Funciona como evidencia copiada de referencia.
- **Copia working:** copia operativa sobre la cual se ejecuta el análisis.
1. Calcula el hash SHA-256 de ambas copias y verifica que coincidan con el original.
1. Detecta y copia los archivos auxiliares asociados si existen: `-wal`, `-shm`, `-journal`.
1. Registra todas las acciones en un log de auditoría encadenado por hash.

**El archivo original nunca es abierto con SQLite por la herramienta.** Solo es leído como flujo de bytes para hashing y copia.

-----

## 3. Modo de análisis

La herramienta ofrece dos modos de apertura de la copia working, que se deben seleccionar antes de cada análisis:

### Modo “solo committed”

La base se abre con el parámetro `immutable=1` de SQLite, que la trata como almacenamiento de solo lectura. SQLite no asocia el archivo WAL ni realiza checkpoint. El analista ve únicamente los registros que ya están persistidos en la base principal.

**Implicancia:** los mensajes recientes que aún no fueron checkpointed desde el WAL hacia la base principal **no aparecen** en este modo.

### Modo “incluye WAL”

La base se abre en modo solo lectura sin `immutable`. SQLite asocia el WAL y puede realizar checkpoint automático sobre la copia working. El analista ve registros de la base principal y del WAL.

**Implicancia:** SQLite puede modificar la copia working durante esta operación. La herramienta re-hashea la copia working después de cada análisis en este modo y registra cualquier cambio en el log de auditoría. La copia pristine permanece intacta.

El modo utilizado en cada análisis se registra en el reporte XLSX, en `case.json` y en el audit log.

-----

## 4. Detección de gaps en secuencias

La herramienta identifica discontinuidades en secuencias de claves primarias autoincrementales y en `sqlite_sequence`.

**Un gap no constituye prueba de borrado.** Las causas posibles incluyen:

- Eliminación de registros por parte del usuario o de la aplicación.
- Transacciones abortadas que reservaron pero no usaron un ID.
- Migraciones de esquema entre versiones de la aplicación.
- Reuso o salto de IDs por parte de la lógica de la aplicación.
- Errores transitorios de la aplicación.

La herramienta reporta el gap, su tamaño y los registros vecinos como contexto. La interpretación queda a cargo del investigador.

-----

## 5. Señales de eliminación o revocación

La herramienta clasifica los hallazgos relacionados con posibles eliminaciones en dos categorías estrictamente separadas.

### Categoría A — Revocaciones declaradas por la aplicación

Mensajes que la propia aplicación marca como revocados mediante códigos de tipo específicos en sus tablas. El significado de estos códigos depende de la versión de la aplicación y se documenta en `docs/SCHEMAS.md` por versión validada.

Esta categoría representa información que la aplicación explícitamente registra como revocación. **No implica recuperación del contenido original** del mensaje revocado.

### Categoría B — Anomalías estructurales deducidas

Patrones detectados por la herramienta que pueden ser compatibles con eliminación, pero también con otras causas:

- Mensajes con cuerpo vacío y metadatos presentes.
- Inconsistencias entre tablas relacionadas (referencias rotas).
- Registros presentes en el WAL pero no en la base principal (solo en modo “incluye WAL”).
- Registros en tablas auxiliares de rastros, cuando la versión de la aplicación los mantiene.

Cada hallazgo en esta categoría se etiqueta con una clasificación de fortaleza de evidencia:

- **weak:** patrón con muchas explicaciones alternativas.
- **moderate:** patrón con explicaciones alternativas pero consistente con eliminación.
- **strong:** patrón con pocas explicaciones alternativas razonables.

**Ningún hallazgo de la categoría B constituye prueba de eliminación.** Todos requieren correlación con otras fuentes.

-----

## 6. Lo que la herramienta NO hace

- **No recupera contenido de páginas marcadas como libres** en la base SQLite (carving del freelist). El borrado en SQLite no sobrescribe inmediatamente los datos, pero la herramienta en su versión actual no extrae registros de páginas libres.
- **No reconstruye frames antiguos del WAL** que hayan sido sobrescritos por checkpoints posteriores.
- **No descifra** backups cifrados, archivos `msgstore.db.crypt14` ni equivalentes. Solo procesa bases SQLite ya descifradas.
- **No realiza análisis de unallocated space** en los archivos.
- **No correlaciona con otras fuentes** como artefactos del sistema operativo, registros de red, o archivos multimedia. El analista debe hacer esa correlación manualmente.
- **No interpreta el significado** de mensajes ni evalúa veracidad de su contenido.
- **No genera firmas digitales** de los reportes. El audit log encadenado por hash provee trazabilidad pero no es equivalente a una firma criptográfica.

-----

## 7. Reproducibilidad

La herramienta está diseñada para producir resultados bit-identical sobre la misma entrada y la misma versión de la herramienta. El reporte XLSX incluye:

- Versión de la herramienta y hash de la build.
- Versión de las dependencias críticas.
- Modo de análisis utilizado.
- Hashes de cada archivo ingresado.

Dos analistas ejecutando la misma versión sobre la misma evidencia deben obtener reportes equivalentes (excepto por la marca temporal de generación).

-----

## 8. Limitaciones de interpretación

- Los timestamps se interpretan según el formato detectado para cada columna (Mac Absolute Time, Unix segundos, Unix milisegundos). Si la detección de formato es ambigua, se reporta el valor crudo además del interpretado.
- La zona horaria aplicada al reporte es la declarada por el investigador al crear el caso, no se infiere automáticamente del contenido de la base.
- Los participantes de una conversación se obtienen de las tablas correspondientes a la versión validada de la aplicación. Cambios de número de teléfono, alias o identificadores entre versiones pueden no estar reflejados.
- Los mensajes multimedia se listan por referencia. La herramienta no verifica la existencia ni integridad del archivo multimedia referenciado.

-----

## 9. Recomendación de uso

Los reportes generados por esta herramienta deben:

1. Ser correlacionados con al menos una fuente independiente antes de fundamentar conclusiones.
1. Ser revisados por un investigador con conocimiento del esquema de la aplicación analizada y de la versión específica involucrada.
1. Mantenerse junto con el workspace del caso (que contiene las copias pristine, el audit log y los hashes) durante todo el ciclo de la investigación.
1. No ser citados como prueba autónoma de eliminación o revocación de información.

-----

## 10. Vocabulario empleado

A lo largo del reporte se utiliza vocabulario deliberadamente prudente:

- **Señal:** patrón observado en los datos.
- **Indicio:** observación que puede sugerir un hecho pero no lo prueba.
- **Anomalía:** desviación respecto a lo esperado.
- **Compatible con:** consistente con una hipótesis, sin descartar otras.
- **Requiere correlación:** necesita verificación con fuentes adicionales.

Se evita deliberadamente el uso de los términos **prueba**, **demuestra** y **confirma** en relación con los hallazgos.
