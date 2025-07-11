import sqlite3
import re
import os
from datetime import datetime
import pandas as pd

# ==================================================================
# whatsapp_forensic_parser.py
# Backend: Enhanced forensic parser for WhatsApp msgstore.db
# Thread-safe: each method uses its own SQLite connection
# Handles missing optional tables/views gracefully
# ==================================================================

RE_NUMBER = re.compile(r"(\d+)")
RE_SANITIZE = re.compile(r"[\\/:*?\"<>|]")
DEFAULT_COUNTRY_CODE = '51'


def load_vcf(vcf_path):
    """Load contacts from .vcf/.txt into dict: digits -> full name."""
    contacts = {}
    if not vcf_path:
        return contacts
    files = []
    if os.path.isdir(vcf_path):
        for fname in os.listdir(vcf_path):
            if fname.lower().endswith(('.vcf', '.txt')):
                files.append(os.path.join(vcf_path, fname))
    else:
        files.append(vcf_path)
    for path in files:
        try:
            text = open(path, encoding='utf-8').read()
        except IOError:
            continue
        for block in text.split('BEGIN:VCARD'):
            if 'FN:' not in block:
                continue
            name = None
            for line in block.splitlines():
                if line.startswith('FN:'):
                    name = line[3:].strip()
                if 'TEL' in line:
                    parts = line.split(':', 1)
                    if len(parts) == 2:
                        digits = re.sub(r'\D', '', parts[1])
                        if digits:
                            contacts[digits] = name
                            if DEFAULT_COUNTRY_CODE and not digits.startswith(DEFAULT_COUNTRY_CODE):
                                contacts[DEFAULT_COUNTRY_CODE + digits] = name
    return contacts


def normalize_number(raw):
    m = RE_NUMBER.match(raw or '')
    return f'+{m.group(1)}' if m else ''


def sanitize_filename(name):
    return RE_SANITIZE.sub('_', name or '')


def format_timestamp(ms):
    try:
        return datetime.fromtimestamp(ms / 1000).strftime('%Y-%m-%d %H:%M:%S')
    except:
        return ''


def _export_chat(raw_jid, display_name, messages, output_dir, writer):
    # Prepare safe filename and number
    num = normalize_number(raw_jid).lstrip('+')
    safe = sanitize_filename(display_name)[:30]
    # Write TXT
    txt_path = os.path.join(output_dir, f"{safe} - {num}.txt")
    with open(txt_path, 'w', encoding='utf-8') as f:
        f.write(f"{display_name} - {num}\n\n")
        for sender, text, ts in messages:
            f.write(f"[{ts}] {sender}: {text}\n")
    # Add Excel sheet
    df = pd.DataFrame([{'Timestamp': ts, 'Sender': sender, 'Message': text} for sender, text, ts in messages])
    df.to_excel(writer, sheet_name=safe, index=False)


class WhatsAppForensicParser:
    def __init__(self, db_path, vcf_path=None):
        self.db_path = db_path
        self.name_map = load_vcf(vcf_path)

    def _connect(self):
        return sqlite3.connect(self.db_path)

    def list_chats(self):
        ind, grp, emp = [], [], []
        sql = (
            "SELECT "
            "  c._id, "
            "  COALESCE(c.subject, '') AS subj, "
            "  j.raw_string AS raw, "
            "  IFNULL(m.cnt, 0) AS cnt "
            "FROM chat AS c "
            "JOIN jid AS j "
            "  ON c.jid_row_id = j._id "
            "LEFT JOIN ("
            "  SELECT chat_row_id, COUNT(*) AS cnt "
            "  FROM message "
            "  GROUP BY chat_row_id"
            ") AS m "
            "  ON c._id = m.chat_row_id "
            "ORDER BY subj, c._id"
        )

        conn = self._connect()
        try:
            for cid, subj, raw, cnt in conn.execute(sql):
                # Dominio del JID (para distinguir grupos .g.us)
                domain = raw.split('@')[1] if '@' in raw else ''

                # Determinamos la etiqueta a mostrar
                if subj:
                    disp = subj
                else:
                    m = RE_NUMBER.match(raw)
                    num = m.group(1) if m else raw
                    disp = self.name_map.get(num, f'+{num}')

                # Clasificamos según número de mensajes y dominio
                if cnt == 0:
                    emp.append((cid, disp))
                elif domain == 'g.us':
                    grp.append((cid, disp))
                else:
                    ind.append((cid, disp))

        finally:
            conn.close()

        return ind, grp, emp


    def get_raw_jid(self, chat_id):
        conn = self._connect()
        try:
            row = conn.execute(
                "SELECT jid.raw_string FROM chat JOIN jid ON chat.jid_row_id=jid._id WHERE chat._id=?", (chat_id,)
            ).fetchone()
            return row[0] if row else ''
        finally:
            conn.close()

    def get_chat_info(self, chat_id):
        conn = self._connect()
        try:
            row = conn.execute(
                "SELECT COALESCE(subject,''), created_timestamp FROM chat WHERE _id=?", (chat_id,)
            ).fetchone()
            subj, ts = row if row else ('', 0)
            total = conn.execute(
                "SELECT COUNT(*) FROM message WHERE chat_row_id=?", (chat_id,)
            ).fetchone()[0]
        finally:
            conn.close()
        # fallback for individual chats
        if not subj:
            raw = self.get_raw_jid(chat_id)
            num = RE_NUMBER.match(raw).group(1) if RE_NUMBER.match(raw) else raw
            subj = self.name_map.get(num, f'+{num}')
        return subj, format_timestamp(ts), total

    def fetch_messages(self, chat_id, offset=0, limit=100):
        msgs = []
        conn = self._connect()
        try:
            for frm, text, ts, sjid in conn.execute(
                "SELECT from_me, text_data, timestamp, sender_jid_row_id "
                "FROM message WHERE chat_row_id=? AND text_data IS NOT NULL "
                "ORDER BY timestamp LIMIT ? OFFSET ?", (chat_id, limit, offset)
            ):
                sender = 'Tú' if frm == 1 else self._resolve_sender(sjid, conn)
                msgs.append((sender, text, format_timestamp(ts)))
        finally:
            conn.close()
        return msgs

    def _resolve_sender(self, sjid, conn):
        row = conn.execute("SELECT raw_string FROM jid WHERE _id=?", (sjid,)).fetchone()
        raw = row[0] if row else ''
        num = RE_NUMBER.match(raw).group(1) if RE_NUMBER.match(raw) else raw
        return self.name_map.get(num, f'+{num}')

    def search_messages_in_chat(self, chat_id, keyword):
        res = []
        conn = self._connect()
        try:
            for frm, text, ts, sjid in conn.execute(
                "SELECT from_me, text_data, timestamp, sender_jid_row_id "
                "FROM message WHERE chat_row_id=? AND text_data LIKE ? "
                "ORDER BY timestamp", (chat_id, f'%{keyword}%')
            ):
                sender = 'Tú' if frm == 1 else self._resolve_sender(sjid, conn)
                res.append((sender, text, format_timestamp(ts)))
        finally:
            conn.close()
        return res

    def search_messages_global(self, keyword, limit=200):
        """Busca mensajes en todos los chats que contengan el keyword."""
        res = []
        conn = self._connect()
        try:
            for chat_id, sender, text, ts in conn.execute(
                "SELECT chat_row_id, from_me, text_data, timestamp, sender_jid_row_id "
                "FROM message WHERE text_data LIKE ? ORDER BY timestamp DESC LIMIT ?",
                (f'%{keyword}%', limit)
            ):
                sender_name = 'Tú' if sender == 1 else self._resolve_sender(ts, conn)
                res.append((chat_id, sender_name, text, format_timestamp(ts)))
        finally:
            conn.close()
        return res

    def get_deleted_messages(self, chat_id):
        try:
            conn = self._connect(); res = []
            for sort_id, frm, text, ts in conn.execute(
                "SELECT sort_id, from_me, text_data, timestamp FROM deleted_message_view WHERE chat_row_id=? ORDER BY timestamp", (chat_id,)
            ):
                sender = 'Tú' if frm == 1 else 'Otro'
                res.append((sort_id, sender, text, format_timestamp(ts)))
            return res
        except sqlite3.OperationalError:
            return []
        finally:
            conn.close()

    def get_available_snapshot(self, chat_id):
        try:
            conn = self._connect(); res = []
            for row in conn.execute(
                "SELECT * FROM available_message_view WHERE chat_row_id=? ORDER BY timestamp", (chat_id,)
            ):
                res.append(row)
            return res
        except sqlite3.OperationalError:
            return []
        finally:
            conn.close()

    def list_labels(self):
        try:
            conn = self._connect(); lbls = []
            for _id, name, pre, color, typ, hidden in conn.execute(
                "SELECT _id,label_name,predefined_id,color_id,type,hidden FROM labels"
            ):
                lbls.append((name, typ, hidden))
            return lbls
        except sqlite3.OperationalError:
            return []
        finally:
            conn.close()

    def get_call_history(self, chat_id=None):
        try:
            conn = self._connect()
            calls = []

            sql = (
                "SELECT "
                "  call_log._id, "
                "  datetime(call_log.timestamp/1000, 'unixepoch', 'localtime') AS hora_llamada, "
                "  printf("
                "    '%02d:%02d:%03d', "
                "    CAST(call_log.duration/60 AS INTEGER), "
                "    CAST(call_log.duration % 60 AS INTEGER), "
                "    ROUND((call_log.duration - CAST(call_log.duration AS INTEGER)) * 1000)"
                "  ) AS duracion_formateada, "
                "  CASE call_log.from_me "
                "    WHEN 1 THEN 'Tú' "
                "    ELSE jid.raw_string "
                "  END AS origen, "
                "  CASE call_log.video_call "
                "    WHEN 1 THEN 'Sí' "
                "    ELSE 'No' "
                "  END AS es_videollamada, "
                "  ROUND(call_log.bytes_transferred / (1024.0 * 1024.0), 2) AS mb_transf_megas, "
                "  CASE call_log.call_result "
                "    WHEN 5 THEN 'Contestada' "
                "    WHEN 4 THEN 'Perdida' "
                "    WHEN 2 THEN CASE WHEN call_log.from_me = 1 THEN 'Cancelada' ELSE 'Rechazada' END "
                "    ELSE 'Desconocido' "
                "  END AS resultado_llamada "
                "FROM call_log "
                "JOIN jid ON call_log.jid_row_id = jid._id"
            )

            params = ()

            for call_id, hora_llamada, duracion, origen, videollamada, peso_mb, resultado_llamada in conn.execute(sql, params):
                calls.append((
                    call_id,
                    origen,
                    hora_llamada,
                    duracion,
                    videollamada,
                    peso_mb,
                    resultado_llamada
                ))

            return calls

        except sqlite3.OperationalError:
            return []

        finally:
            conn.close()


    def get_system_events(self, chat_id):
        evts = []
        conn = self._connect()
        try:
            # system value changes
            try:
                for mid, old in conn.execute("SELECT message_row_id,old_data FROM message_system_value_change"):
                    evts.append((mid, 'ValueChange', old))
            except sqlite3.OperationalError:
                pass
            # group join/leave
            try:
                for mid, joined in conn.execute("SELECT message_row_id,is_me_joined FROM message_system_group"):
                    evts.append((mid, 'GroupEvent', joined))
            except sqlite3.OperationalError:
                pass
            # number changes
            try:
                for mid, old, new in conn.execute(
                    "SELECT message_row_id,old_jid_row_id,new_jid_row_id FROM message_system_number_change"
                ):
                    evts.append((mid, 'NumberChange', (old, new)))
            except sqlite3.OperationalError:
                pass
            return evts
        finally:
            conn.close()

    def get_mentions(self, chat_id):
        """
        Devuelve las menciones de un chat (o de todos si chat_id es None),
        con: Chat (grupo o número), Momento de la mención, Quién la hizo y A quién se menciona.
        """
        mentions = []
        conn = self._connect()
        try:
            try:
                sql = """
                SELECT
                -- Chat (nombre de grupo o número de contacto)
                COALESCE(
                    c.subject,
                    substr(COALESCE(chat_jc.raw_string, ''), 1,
                        instr(COALESCE(chat_jc.raw_string, ''), '@') - 1)
                ) AS chat,

                -- Momento de la mención
                datetime(m.timestamp/1000, 'unixepoch','localtime') AS momento,

                -- Quién envió el mensaje
                CASE
                    WHEN m.from_me = 1 THEN 'Tú'
                    ELSE substr(COALESCE(js.raw_string, ''), 1,
                                instr(COALESCE(js.raw_string, ''), '@') - 1)
                END AS quien,

                -- A quién se menciona
                CASE
                    WHEN COALESCE(mm.display_name, '') <> ''
                    THEN mm.display_name
                    ELSE substr(COALESCE(jm.raw_string, ''), 1,
                                instr(COALESCE(jm.raw_string, ''), '@') - 1)
                END AS mencionado

                FROM message_mentions AS mm
                JOIN message          AS m   ON mm.message_row_id       = m._id
                LEFT JOIN chat        AS c   ON m.chat_row_id           = c._id
                LEFT JOIN jid         AS chat_jc ON c.jid_row_id        = chat_jc._id
                LEFT JOIN jid         AS js  ON m.sender_jid_row_id     = js._id
                LEFT JOIN jid         AS jm  ON mm.jid_row_id           = jm._id

                WHERE (? IS NULL OR m.chat_row_id = ?)
                ORDER BY m.timestamp DESC;
                """
                params = (chat_id, chat_id)
                for chat, momento, quien, mencionado in conn.execute(sql, params):
                    mentions.append((chat, momento, quien, mencionado))
            except sqlite3.OperationalError:
                # En caso de que no exista message_mentions en versiones antiguas
                pass

            return mentions
        finally:
            conn.close()


    def get_links(self, chat_id):
        lks = []
        conn = self._connect()
        try:
            try:
                for _id, mid, lidx in conn.execute(
                    "SELECT _id,message_row_id,link_index FROM message_link WHERE chat_row_id=?", (chat_id,)
                ):
                    lks.append((mid, lidx))
            except sqlite3.OperationalError:
                pass
            return lks
        finally:
            conn.close()

    def get_edit_history(self, chat_id):
        """
        Devuelve el historial de ediciones para un chat específico,
        incluyendo contacto/grupo, texto original y editado, fechas
        y retraso en segundos.
        """
        edits = []
        conn = self._connect()
        try:
            try:
                sql = """
                SELECT
                -- Nombre de grupo o número de contacto
                COALESCE(
                    c.subject,
                    substr(j.raw_string, 1, instr(j.raw_string, '@') - 1)
                ) AS contact_group,
    
                -- Datos del mensaje original
                mei.original_key_id       AS original_id,
                orig.text_data            AS original_text,
                datetime(
                    mei.sender_timestamp/1000,
                    'unixepoch','localtime'
                )                          AS fecha_envio,
    
                -- Datos del mensaje editado
                mei.message_row_id        AS edited_id,
                edit.text_data            AS edited_text,
                datetime(
                    mei.edited_timestamp/1000,
                    'unixepoch','localtime'
                )                          AS fecha_edicion,
    
                -- Retraso entre envío y edición (s)
                ROUND(
                    (mei.edited_timestamp - mei.sender_timestamp) / 1000.0,
                    2
                )                          AS retraso_s
                FROM
                message_edit_info AS mei
                LEFT JOIN message AS orig
                    ON orig.key_id = mei.original_key_id
                LEFT JOIN message AS edit
                    ON edit._id = mei.message_row_id
                LEFT JOIN chat AS c
                    ON edit.chat_row_id = c._id
                LEFT JOIN jid AS j
                    ON c.jid_row_id = j._id
                WHERE
                c._id = ?
                ORDER BY
                mei.edited_timestamp DESC
                """
                for row in conn.execute(sql, (chat_id,)):
                    edits.append(row)
            except sqlite3.OperationalError:
                # Tabla message_edit_info puede no existir en versiones antiguas
                pass

            return edits
        finally:
            conn.close()


    def export_all(self, output_dir):
        os.makedirs(output_dir, exist_ok=True)
        writer = pd.ExcelWriter(os.path.join(output_dir, 'whatsapp_forensic.xlsx'), engine='openpyxl')
        stats = []
        ind, grp, emp = self.list_chats()
        for cid, disp in ind + grp + emp:
            raw = self.get_raw_jid(cid)
            msgs = self.fetch_messages(cid, 0, 10**9)
            _export_chat(raw, disp, msgs, output_dir, writer)
            stats.append({'Chat': disp, 'Total': len(msgs)})
        pd.DataFrame(stats).to_excel(writer, sheet_name='Stats', index=False)
        writer.save()
