# Esquemas verificados por versión

Este documento registra los esquemas de aplicaciones que han sido **verificados empíricamente contra evidencia real**. No se debe agregar entradas a este documento por inferencia, memoria o documentación de terceros sin verificación directa.

Cada entrada debe incluir:

- Fuente (aplicación + plataforma).
- Versión exacta verificada (cuando esté disponible).
- Fecha de verificación.
- Quién verificó.
- Hash SHA-256 de la evidencia de referencia utilizada para verificar.
- Tablas y columnas presentes.
- Mapeos de códigos confirmados.

Los mapeos derivados de este documento se implementan como archivos TOML en `src-tauri/resources/schemas/<source>/<version>.toml`.

-----

## Estado actual

**Ninguna versión verificada todavía.** Este documento se completa durante las fases 3 y 4.

-----

## Plantilla por entrada

```
### <Source>: <App> <Plataforma>

**Versión verificada:** <X.Y.Z>
**Fecha de verificación:** YYYY-MM-DD
**Verificado por:** <nombre>
**Evidencia de referencia (SHA-256):** <hash>
**Origen de la evidencia:** <descripción genérica, sin datos personales>

#### Tablas relevantes

- `<TABLE_NAME>`: <descripción>
  - Columnas verificadas: ...
  - Columnas no presentes en esta versión: ...

#### Mapeos de códigos confirmados

| Columna | Valor crudo | Significado | Fuente de la confirmación |
|---|---|---|---|
| ZMESSAGETYPE | 0 | text | observación empírica + cross-check con N mensajes |
| ZMESSAGETYPE | 7 | revoked | observación empírica + UI muestra "mensaje eliminado" |

#### Notas

- Diferencias respecto a versiones anteriores: ...
- Comportamientos inusuales observados: ...
- Limitaciones del esquema: ...
```

-----

## Reglas para mantener este documento

1. **No agregar entradas sin verificación empírica.** Si no se confirmó contra una base real, no se documenta.
1. **No copiar esquemas de terceros sin verificar.** Documentación externa sirve como guía, no como fuente.
1. **Versiones distintas requieren entradas distintas.** Los esquemas cambian entre versiones de la misma aplicación.
1. **Si una columna está presente pero su significado no se confirmó, documentarlo así.** Mejor “presente, significado no confirmado” que omitirlo o inventar significado.
1. **Cuando se invalida un mapeo previo**, no borrarlo: marcarlo como obsoleto con fecha y razón.

-----

## Fuentes esperadas

A medida que se vayan verificando, se documentarán entradas para:

- WhatsApp iOS — `ChatStorage.sqlite`
- WhatsApp Android — `msgstore.db`
- CallHistory iOS — `CallHistory.storedata`
- CallHistory iOS — `CallHistory.sqlite`
- Otras fuentes a determinar
