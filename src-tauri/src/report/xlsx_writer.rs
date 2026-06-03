//! Generador del reporte XLSX. Reproducibilidad bit-identical.
//!
//! Embebe el texto de `docs/METHODOLOGY.md` literal en su propia hoja.
//! Ordena resultados explícitamente. Usa `BTreeMap` para estabilidad.

use crate::analysis::AnalysisFindings;
use crate::commands::case_cmd::CaseSummary;
use crate::error::{AppError, AppErrorKind};
use crate::parsers::common_model::{MessageDirection, ParsedEvidence};
use crate::workspace::audit_log::AuditEntry;
use crate::workspace::manifest::EvidenceEntry;
use rust_xlsxwriter::{Format, FormatAlign, FormatBorder, Workbook};
use std::path::Path;

const METHODOLOGY_MD: &str = include_str!("../../../docs/METHODOLOGY.md");

pub struct ExportOptions {
    pub include_raw_row_json: bool,
    pub timezone: String,
}

pub struct ExportInputs<'a> {
    pub case: &'a CaseSummary,
    pub evidence: &'a EvidenceEntry,
    pub parsed: &'a ParsedEvidence,
    pub findings: &'a AnalysisFindings,
    pub audit_entries: &'a [AuditEntry],
}

pub fn export(
    inputs: ExportInputs<'_>,
    output_path: &Path,
    options: &ExportOptions,
) -> Result<(), AppError> {
    let mut wb = Workbook::new();

    let title_fmt = Format::new()
        .set_bold()
        .set_font_size(14.0)
        .set_align(FormatAlign::Left);
    let label_fmt = Format::new().set_bold().set_font_color("#444444");
    let mono_fmt = Format::new().set_font_name("Consolas").set_font_size(9.0);
    let warn_fmt = Format::new()
        .set_background_color("#FEF3C7")
        .set_font_color("#92400E");
    let header_fmt = Format::new()
        .set_bold()
        .set_background_color("#E5E7EB")
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Left);

    // ----------------- Portada -----------------
    {
        let sheet = wb.add_worksheet().set_name("Portada").map_err(map_err)?;
        let _ = sheet.set_column_width(0, 32);
        let _ = sheet.set_column_width(1, 60);
        let mut row = 0u32;
        sheet
            .write_with_format(row, 0, "WhatsForensics — Reporte forense", &title_fmt)
            .map_err(map_err)?;
        row += 2;
        write_kv(sheet, &mut row, &label_fmt, "Herramienta", "WhatsForensics")?;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Versión de la herramienta",
            env!("CARGO_PKG_VERSION"),
        )?;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Build hash",
            option_env!("WF_BUILD_HASH").unwrap_or("untracked"),
        )?;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Generado (UTC)",
            &chrono::Utc::now().to_rfc3339(),
        )?;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Zona horaria del caso",
            &options.timezone,
        )?;
        row += 1;
        write_kv(sheet, &mut row, &label_fmt, "Caso", &inputs.case.name)?;
        write_kv(sheet, &mut row, &label_fmt, "Case ID", &inputs.case.case_id)?;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Investigador",
            &inputs.case.investigator,
        )?;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Creado (UTC)",
            &inputs.case.created_at.to_rfc3339(),
        )?;
        if let Some(d) = &inputs.case.description {
            write_kv(sheet, &mut row, &label_fmt, "Descripción", d)?;
        }
        row += 1;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Evidencia",
            &inputs.evidence.filename,
        )?;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Evidence ID",
            &inputs.evidence.evidence_id,
        )?;
        if !inputs.parsed.schema_verified {
            row += 1;
            sheet
                .write_with_format(
                    row,
                    0,
                    "AVISO: El parser utilizó un mapping seed con verified=false.",
                    &warn_fmt,
                )
                .map_err(map_err)?;
            row += 1;
            sheet
                .write_with_format(
                    row,
                    0,
                    "Las interpretaciones de tipos de mensaje son orientativas y requieren verificación.",
                    &warn_fmt,
                )
                .map_err(map_err)?;
        }
    }

    // ----------------- Evidencia -----------------
    {
        let sheet = wb.add_worksheet().set_name("Evidencia").map_err(map_err)?;
        let _ = sheet.set_column_width(0, 32);
        let _ = sheet.set_column_width(1, 72);
        let mut row = 0u32;
        sheet
            .write_with_format(row, 0, "Hashes y modo de análisis", &title_fmt)
            .map_err(map_err)?;
        row += 2;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Archivo",
            &inputs.evidence.filename,
        )?;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Path original",
            &inputs.evidence.original_path,
        )?;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Tamaño original (bytes)",
            &inputs.evidence.original_size.to_string(),
        )?;
        write_kv_mono(
            sheet,
            &mut row,
            &label_fmt,
            &mono_fmt,
            "SHA-256 original",
            &inputs.evidence.original_sha256,
        )?;
        write_kv_mono(
            sheet,
            &mut row,
            &label_fmt,
            &mono_fmt,
            "SHA-256 pristine",
            &inputs.evidence.pristine_sha256,
        )?;
        write_kv_mono(
            sheet,
            &mut row,
            &label_fmt,
            &mono_fmt,
            "SHA-256 working",
            &inputs.evidence.working_sha256,
        )?;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Sidecars detectados",
            &format!(
                "WAL={} SHM={} JOURNAL={}",
                yes_no(inputs.evidence.has_wal),
                yes_no(inputs.evidence.has_shm),
                yes_no(inputs.evidence.has_journal)
            ),
        )?;
        for (sname, shash) in &inputs.evidence.sidecar_hashes {
            write_kv_mono(
                sheet,
                &mut row,
                &label_fmt,
                &mono_fmt,
                &format!("SHA-256 {sname}"),
                shash,
            )?;
        }
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Modo de análisis",
            inputs
                .evidence
                .analysis_mode_used
                .as_deref()
                .unwrap_or("no aplicado"),
        )?;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Ingresada (UTC)",
            &inputs.evidence.ingested_at.to_rfc3339(),
        )?;
    }

    // ----------------- Resumen ejecutivo -----------------
    {
        let sheet = wb
            .add_worksheet()
            .set_name("Resumen ejecutivo")
            .map_err(map_err)?;
        let _ = sheet.set_column_width(0, 36);
        let _ = sheet.set_column_width(1, 24);
        let mut row = 0u32;
        sheet
            .write_with_format(row, 0, "Resumen ejecutivo", &title_fmt)
            .map_err(map_err)?;
        row += 2;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Fuente detectada",
            &inputs.parsed.source_kind,
        )?;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Mapping usado",
            &format!(
                "{} (verificado: {})",
                inputs.parsed.schema_version_used,
                yes_no(inputs.parsed.schema_verified)
            ),
        )?;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Conversaciones",
            &inputs.parsed.conversations.len().to_string(),
        )?;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Mensajes",
            &inputs.parsed.messages.len().to_string(),
        )?;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Llamadas",
            &inputs.parsed.calls.len().to_string(),
        )?;
        let revoked = inputs
            .parsed
            .messages
            .iter()
            .filter(|m| m.is_possibly_revoked)
            .count();
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Mensajes con señal de revocación",
            &revoked.to_string(),
        )?;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Gaps detectados",
            &inputs.findings.gaps.len().to_string(),
        )?;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Hallazgos categoría A+B",
            &inputs.findings.deleted_hints.len().to_string(),
        )?;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Warnings del parser",
            &inputs.parsed.warnings.len().to_string(),
        )?;
    }

    // ----------------- Metodología (literal de METHODOLOGY.md) -----------------
    {
        let sheet = wb
            .add_worksheet()
            .set_name("Metodología")
            .map_err(map_err)?;
        let _ = sheet.set_column_width(0, 110);
        for (i, line) in METHODOLOGY_MD.lines().enumerate() {
            sheet.write(i as u32, 0, line).map_err(map_err)?;
        }
    }

    // ----------------- Conversaciones -----------------
    {
        let sheet = wb
            .add_worksheet()
            .set_name("Conversaciones")
            .map_err(map_err)?;
        let cols = [
            "ID",
            "Display name",
            "Tabla origen",
            "Source PK",
            "Primer mensaje (UTC)",
            "Último mensaje (UTC)",
            "Cantidad de mensajes",
        ];
        write_headers(sheet, &cols, &header_fmt)?;
        for (i, c) in inputs.parsed.conversations.iter().enumerate() {
            let r = (i + 1) as u32;
            sheet.write(r, 0, &c.id).map_err(map_err)?;
            sheet
                .write(r, 1, c.display_name.as_deref().unwrap_or(""))
                .map_err(map_err)?;
            sheet.write(r, 2, &c.source_table).map_err(map_err)?;
            sheet.write(r, 3, c.source_pk).map_err(map_err)?;
            sheet
                .write(
                    r,
                    4,
                    c.first_seen_utc.map(|t| t.to_rfc3339()).unwrap_or_default(),
                )
                .map_err(map_err)?;
            sheet
                .write(
                    r,
                    5,
                    c.last_seen_utc.map(|t| t.to_rfc3339()).unwrap_or_default(),
                )
                .map_err(map_err)?;
            sheet.write(r, 6, c.message_count as f64).map_err(map_err)?;
        }
        let _ = sheet.autofilter(
            0,
            0,
            inputs.parsed.conversations.len() as u32,
            (cols.len() - 1) as u16,
        );
        let _ = sheet.set_freeze_panes(1, 0);
    }

    // ----------------- Mensajes -----------------
    {
        let sheet = wb.add_worksheet().set_name("Mensajes").map_err(map_err)?;
        let mut cols = vec![
            "Source PK",
            "Conversation ID",
            "Timestamp (UTC)",
            "Timestamp crudo",
            "Formato crudo",
            "Dirección",
            "Sender",
            "Tipo (crudo)",
            "Tipo (interpretado)",
            "Tipo verificado",
            "Señal de revocación",
            "Cuerpo (texto)",
        ];
        if options.include_raw_row_json {
            cols.push("raw_row JSON");
        }
        write_headers(sheet, &cols, &header_fmt)?;
        for (i, m) in inputs.parsed.messages.iter().enumerate() {
            let r = (i + 1) as u32;
            sheet.write(r, 0, m.source_pk).map_err(map_err)?;
            sheet.write(r, 1, &m.conversation_id).map_err(map_err)?;
            sheet
                .write(
                    r,
                    2,
                    m.timestamp_utc.map(|t| t.to_rfc3339()).unwrap_or_default(),
                )
                .map_err(map_err)?;
            sheet
                .write(r, 3, m.timestamp_raw.unwrap_or(0.0))
                .map_err(map_err)?;
            sheet
                .write(r, 4, m.timestamp_raw_format.as_deref().unwrap_or(""))
                .map_err(map_err)?;
            sheet.write(r, 5, dir_str(m.direction)).map_err(map_err)?;
            sheet
                .write(r, 6, m.sender.as_deref().unwrap_or(""))
                .map_err(map_err)?;
            sheet
                .write(r, 7, m.message_type_raw.map(|v| v as f64).unwrap_or(-1.0))
                .map_err(map_err)?;
            sheet
                .write(r, 8, m.message_type_interpreted.as_deref().unwrap_or(""))
                .map_err(map_err)?;
            sheet
                .write(r, 9, yes_no(m.message_type_verified))
                .map_err(map_err)?;
            sheet
                .write(r, 10, yes_no(m.is_possibly_revoked))
                .map_err(map_err)?;
            sheet
                .write(r, 11, m.body.as_deref().unwrap_or(""))
                .map_err(map_err)?;
            if options.include_raw_row_json {
                let raw = serde_json::to_string(&m.raw_row).unwrap_or_default();
                sheet.write(r, 12, &raw).map_err(map_err)?;
            }
        }
        let _ = sheet.autofilter(
            0,
            0,
            inputs.parsed.messages.len() as u32,
            (cols.len() - 1) as u16,
        );
        let _ = sheet.set_freeze_panes(1, 0);
    }

    // ----------------- Llamadas -----------------
    if !inputs.parsed.calls.is_empty() {
        let sheet = wb.add_worksheet().set_name("Llamadas").map_err(map_err)?;
        let cols = [
            "Source PK",
            "Timestamp (UTC)",
            "Timestamp crudo",
            "Dirección",
            "Peer",
            "Duración (s)",
            "Tipo (crudo)",
            "Tipo (interpretado)",
            "Tipo verificado",
        ];
        write_headers(sheet, &cols, &header_fmt)?;
        for (i, c) in inputs.parsed.calls.iter().enumerate() {
            let r = (i + 1) as u32;
            sheet.write(r, 0, c.source_pk).map_err(map_err)?;
            sheet
                .write(
                    r,
                    1,
                    c.timestamp_utc.map(|t| t.to_rfc3339()).unwrap_or_default(),
                )
                .map_err(map_err)?;
            sheet
                .write(r, 2, c.timestamp_raw.unwrap_or(0.0))
                .map_err(map_err)?;
            sheet.write(r, 3, dir_str(c.direction)).map_err(map_err)?;
            sheet
                .write(r, 4, c.peer.as_deref().unwrap_or(""))
                .map_err(map_err)?;
            sheet
                .write(r, 5, c.duration_seconds.map(|v| v as f64).unwrap_or(-1.0))
                .map_err(map_err)?;
            sheet
                .write(r, 6, c.call_type_raw.map(|v| v as f64).unwrap_or(-1.0))
                .map_err(map_err)?;
            sheet
                .write(r, 7, c.call_type_interpreted.as_deref().unwrap_or(""))
                .map_err(map_err)?;
            sheet
                .write(r, 8, yes_no(c.call_type_verified))
                .map_err(map_err)?;
        }
        let _ = sheet.autofilter(
            0,
            0,
            inputs.parsed.calls.len() as u32,
            (cols.len() - 1) as u16,
        );
        let _ = sheet.set_freeze_panes(1, 0);
    }

    // ----------------- Gaps -----------------
    {
        let sheet = wb.add_worksheet().set_name("Gaps").map_err(map_err)?;
        let cols = [
            "Tabla",
            "Columna",
            "Inicio",
            "Fin",
            "Tamaño",
            "PK previo",
            "PK siguiente",
            "Origen",
            "Interpretación",
        ];
        write_headers(sheet, &cols, &header_fmt)?;
        for (i, g) in inputs.findings.gaps.iter().enumerate() {
            let r = (i + 1) as u32;
            sheet.write(r, 0, &g.table).map_err(map_err)?;
            sheet.write(r, 1, &g.column).map_err(map_err)?;
            sheet.write(r, 2, g.range_start).map_err(map_err)?;
            sheet.write(r, 3, g.range_end).map_err(map_err)?;
            sheet.write(r, 4, g.size).map_err(map_err)?;
            sheet
                .write(r, 5, g.prev_pk.unwrap_or(-1))
                .map_err(map_err)?;
            sheet
                .write(r, 6, g.next_pk.unwrap_or(-1))
                .map_err(map_err)?;
            let src = match g.source {
                crate::analysis::gaps::GapSource::PkSequence => "pk_sequence",
                crate::analysis::gaps::GapSource::SqliteSequenceTail => "sqlite_sequence_tail",
            };
            sheet.write(r, 7, src).map_err(map_err)?;
            sheet.write(r, 8, &g.interpretation_note).map_err(map_err)?;
        }
        let _ = sheet.autofilter(
            0,
            0,
            inputs.findings.gaps.len() as u32,
            (cols.len() - 1) as u16,
        );
        let _ = sheet.set_freeze_panes(1, 0);
    }

    // ----------------- Revocaciones (Categoría A) -----------------
    {
        let sheet = wb
            .add_worksheet()
            .set_name("Revocaciones (A)")
            .map_err(map_err)?;
        let cols = ["Message ID", "Kind", "Evidence strength", "Nota"];
        write_headers(sheet, &cols, &header_fmt)?;
        let mut r = 1u32;
        for h in inputs
            .findings
            .deleted_hints
            .iter()
            .filter(|h| matches!(h.category, crate::analysis::deleted_hints::HintCategory::A))
        {
            sheet.write(r, 0, &h.message_id).map_err(map_err)?;
            sheet.write(r, 1, &h.kind).map_err(map_err)?;
            sheet
                .write(r, 2, strength_str(h.evidence_strength))
                .map_err(map_err)?;
            sheet.write(r, 3, &h.note).map_err(map_err)?;
            r += 1;
        }
        let _ = sheet.autofilter(0, 0, r.saturating_sub(1), (cols.len() - 1) as u16);
        let _ = sheet.set_freeze_panes(1, 0);
    }

    // ----------------- Anomalías estructurales (Categoría B) -----------------
    {
        let sheet = wb
            .add_worksheet()
            .set_name("Anomalías (B)")
            .map_err(map_err)?;
        let cols = ["Message ID", "Kind", "Evidence strength", "Nota"];
        write_headers(sheet, &cols, &header_fmt)?;
        let mut r = 1u32;
        for h in inputs
            .findings
            .deleted_hints
            .iter()
            .filter(|h| matches!(h.category, crate::analysis::deleted_hints::HintCategory::B))
        {
            sheet.write(r, 0, &h.message_id).map_err(map_err)?;
            sheet.write(r, 1, &h.kind).map_err(map_err)?;
            sheet
                .write(r, 2, strength_str(h.evidence_strength))
                .map_err(map_err)?;
            sheet.write(r, 3, &h.note).map_err(map_err)?;
            r += 1;
        }
        let _ = sheet.autofilter(0, 0, r.saturating_sub(1), (cols.len() - 1) as u16);
        let _ = sheet.set_freeze_panes(1, 0);
    }

    // ----------------- Estadísticas -----------------
    {
        let sheet = wb
            .add_worksheet()
            .set_name("Estadísticas")
            .map_err(map_err)?;
        let _ = sheet.set_column_width(0, 36);
        let _ = sheet.set_column_width(1, 16);
        let stats = &inputs.findings.stats;
        let mut row = 0u32;
        sheet
            .write_with_format(row, 0, "Totales", &title_fmt)
            .map_err(map_err)?;
        row += 2;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Conversaciones",
            &stats.total_conversations.to_string(),
        )?;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Mensajes",
            &stats.total_messages.to_string(),
        )?;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Llamadas",
            &stats.total_calls.to_string(),
        )?;
        write_kv(
            sheet,
            &mut row,
            &label_fmt,
            "Con señal de revocación",
            &stats.total_revoked.to_string(),
        )?;
        if let Some(t) = stats.first_message_utc {
            write_kv(
                sheet,
                &mut row,
                &label_fmt,
                "Primer mensaje (UTC)",
                &t.to_rfc3339(),
            )?;
        }
        if let Some(t) = stats.last_message_utc {
            write_kv(
                sheet,
                &mut row,
                &label_fmt,
                "Último mensaje (UTC)",
                &t.to_rfc3339(),
            )?;
        }
        row += 2;
        write_distribution(
            sheet,
            &mut row,
            &label_fmt,
            &header_fmt,
            "Mensajes por día",
            &stats.messages_per_day,
        )?;
        write_distribution(
            sheet,
            &mut row,
            &label_fmt,
            &header_fmt,
            "Mensajes por tipo",
            &stats.messages_per_type,
        )?;
        write_distribution(
            sheet,
            &mut row,
            &label_fmt,
            &header_fmt,
            "Mensajes por dirección",
            &stats.messages_per_direction,
        )?;
        write_distribution(
            sheet,
            &mut row,
            &label_fmt,
            &header_fmt,
            "Mensajes por conversación",
            &stats.messages_per_conversation,
        )?;
        write_distribution(
            sheet,
            &mut row,
            &label_fmt,
            &header_fmt,
            "Llamadas por tipo",
            &stats.calls_per_type,
        )?;
        write_distribution(
            sheet,
            &mut row,
            &label_fmt,
            &header_fmt,
            "Llamadas por dirección",
            &stats.calls_per_direction,
        )?;
    }

    // ----------------- Schema detectado -----------------
    {
        let sheet = wb.add_worksheet().set_name("Schema").map_err(map_err)?;
        let cols = ["Tabla", "Tipo", "Columnas", "Row count"];
        write_headers(sheet, &cols, &header_fmt)?;
        // No tenemos schema acá; listamos la primera tabla origen del modelo.
        if let Some(c) = inputs.parsed.conversations.first() {
            let r = 1u32;
            sheet.write(r, 0, &c.source_table).map_err(map_err)?;
            sheet.write(r, 1, "table").map_err(map_err)?;
            sheet
                .write(r, 2, "(derivado del parser)")
                .map_err(map_err)?;
            sheet.write(r, 3, "").map_err(map_err)?;
        }
        let _ = sheet.set_freeze_panes(1, 0);
    }

    // ----------------- Audit log resumido -----------------
    {
        let sheet = wb.add_worksheet().set_name("Audit").map_err(map_err)?;
        let cols = ["Seq", "UTC", "Actor", "Action", "Prev hash", "Line hash"];
        write_headers(sheet, &cols, &header_fmt)?;
        for (i, e) in inputs.audit_entries.iter().enumerate() {
            let r = (i + 1) as u32;
            sheet.write(r, 0, e.seq as f64).map_err(map_err)?;
            sheet.write(r, 1, e.ts_utc.to_rfc3339()).map_err(map_err)?;
            sheet.write(r, 2, &e.actor).map_err(map_err)?;
            sheet.write(r, 3, &e.action).map_err(map_err)?;
            sheet
                .write_with_format(r, 4, &e.prev_hash, &mono_fmt)
                .map_err(map_err)?;
            sheet
                .write_with_format(r, 5, &e.line_hash, &mono_fmt)
                .map_err(map_err)?;
        }
        let _ = sheet.autofilter(
            0,
            0,
            inputs.audit_entries.len() as u32,
            (cols.len() - 1) as u16,
        );
        let _ = sheet.set_freeze_panes(1, 0);
    }

    wb.save(output_path).map_err(map_err)?;
    Ok(())
}

fn write_kv(
    sheet: &mut rust_xlsxwriter::Worksheet,
    row: &mut u32,
    label_fmt: &Format,
    key: &str,
    value: &str,
) -> Result<(), AppError> {
    sheet
        .write_with_format(*row, 0, key, label_fmt)
        .map_err(map_err)?;
    sheet.write(*row, 1, value).map_err(map_err)?;
    *row += 1;
    Ok(())
}

fn write_kv_mono(
    sheet: &mut rust_xlsxwriter::Worksheet,
    row: &mut u32,
    label_fmt: &Format,
    mono_fmt: &Format,
    key: &str,
    value: &str,
) -> Result<(), AppError> {
    sheet
        .write_with_format(*row, 0, key, label_fmt)
        .map_err(map_err)?;
    sheet
        .write_with_format(*row, 1, value, mono_fmt)
        .map_err(map_err)?;
    *row += 1;
    Ok(())
}

fn write_headers(
    sheet: &mut rust_xlsxwriter::Worksheet,
    cols: &[&str],
    fmt: &Format,
) -> Result<(), AppError> {
    for (i, c) in cols.iter().enumerate() {
        sheet
            .write_with_format(0, i as u16, *c, fmt)
            .map_err(map_err)?;
    }
    Ok(())
}

fn write_distribution<V: ToString>(
    sheet: &mut rust_xlsxwriter::Worksheet,
    row: &mut u32,
    label_fmt: &Format,
    header_fmt: &Format,
    title: &str,
    dist: &std::collections::BTreeMap<String, V>,
) -> Result<(), AppError> {
    sheet
        .write_with_format(*row, 0, title, label_fmt)
        .map_err(map_err)?;
    *row += 1;
    sheet
        .write_with_format(*row, 0, "Clave", header_fmt)
        .map_err(map_err)?;
    sheet
        .write_with_format(*row, 1, "Cantidad", header_fmt)
        .map_err(map_err)?;
    *row += 1;
    for (k, v) in dist {
        sheet.write(*row, 0, k.as_str()).map_err(map_err)?;
        sheet.write(*row, 1, v.to_string()).map_err(map_err)?;
        *row += 1;
    }
    *row += 1;
    Ok(())
}

fn yes_no(b: bool) -> &'static str {
    if b {
        "Sí"
    } else {
        "No"
    }
}

fn dir_str(d: MessageDirection) -> &'static str {
    match d {
        MessageDirection::Incoming => "incoming",
        MessageDirection::Outgoing => "outgoing",
        MessageDirection::Unknown => "unknown",
    }
}

fn strength_str(s: crate::analysis::deleted_hints::EvidenceStrength) -> &'static str {
    use crate::analysis::deleted_hints::EvidenceStrength::*;
    match s {
        Weak => "weak",
        Moderate => "moderate",
        Strong => "strong",
    }
}

fn map_err<E: std::fmt::Display>(e: E) -> AppError {
    AppError::new(
        AppErrorKind::Internal,
        "XLSX_WRITE_FAILED",
        format!("Falla escribiendo XLSX: {e}"),
    )
}
