# Fixtures sintéticos

Esta carpeta contiene **únicamente** fixtures generados de forma sintética.
**Está prohibido por convención del proyecto commitear evidencia real acá**
(ver `CLAUDE.md` root, regla 10).

## Estructura

- `gen/` — crate Rust que genera los fixtures de forma reproducible.
- `output/` — destino por defecto de los archivos generados (ignorado por git).

## Generar los fixtures

Desde `tests/fixtures/gen/`:

```bash
cargo run
```

Por defecto produce los archivos en `../output/`. Para usar otro directorio:

```bash
cargo run -- /ruta/al/output
```

## Fixtures producidos

| Archivo | Patrón emulado | Uso en tests |
|---|---|---|
| `mock-whatsapp-ios.sqlite` | Core Data + tablas `ZWA*`, timestamps Mac Absolute | Parser WhatsApp iOS (fase 3) |
| `mock-whatsapp-android.db` | Tablas `chat`/`message`/`message_media`, timestamps Unix ms | Parser WhatsApp Android (fase 4) |
| `mock-with-wal.db` (+ `-wal`, `-shm`) | Filas en main DB + filas solo en WAL sin checkpoint | Modos de apertura SQLite (fase 2) |
| `mock-with-gaps.db` | PKs `[1,2,3,5,6,9,10]` con `sqlite_sequence.seq = 10` | Detección de gaps (fase 5) |

Los datos son **completamente sintéticos**. Ningún número, identificador
o contenido corresponde a evidencia real ni a un caso conocido.
