import tkinter as tk
from tkinter import filedialog, messagebox, ttk
from whatsapp_forensic_parser import WhatsAppForensicParser, format_timestamp

class ForensicAnalyzer(tk.Tk):
    """WhatsApp Forensic Analyzer with optimized code and enhanced UI"""
    def __init__(self):
        super().__init__()
        self.title("WhatsApp Forensic Analyzer")
        self.state('zoomed')
        self.parser = None
        self.current_chat = None
        self.event_types = ["Eliminados", "Menciones", "Enlaces", "Ediciones", "Snapshot", "Etiquetas"]

        self._create_menu()
        self._create_toolbar()
        ttk.Separator(self, orient='horizontal').pack(fill=tk.X, pady=(0,5))

        # Split main area
        main = ttk.Panedwindow(self, orient=tk.HORIZONTAL)
        main.pack(fill=tk.BOTH, expand=True)
        self._create_chat_list(main)
        self._create_content_panes(main)

        # Status bar
        self.status = tk.StringVar(value="Listo")
        status_bar = ttk.Label(self, textvariable=self.status, relief='sunken', anchor='w')
        status_bar.pack(fill=tk.X)

        self.load_data()

    def _create_menu(self):
        menubar = tk.Menu(self)
        filemenu = tk.Menu(menubar, tearoff=0)
        filemenu.add_command(label="Cargar datos", command=self.load_data)
        filemenu.add_separator()
        filemenu.add_command(label="Salir", command=self.quit)
        menubar.add_cascade(label="Archivo", menu=filemenu)
        self.config(menu=menubar)

    def _create_toolbar(self):
        toolbar = ttk.Frame(self)
        toolbar.pack(side=tk.TOP, fill=tk.X, padx=5, pady=5)
        for text, cmd in [("Exportar chat", self.export_chat),
                          ("Exportar todo", self.export_all)]:
            ttk.Button(toolbar, text=text, command=cmd).pack(side=tk.LEFT, padx=5)

        ttk.Label(toolbar, text="Ver evento:").pack(side=tk.LEFT, padx=(20,2))
        self.event_var = tk.StringVar(value=self.event_types[0])
        cmb = ttk.Combobox(toolbar, values=self.event_types, textvariable=self.event_var,
                           state='readonly', width=12)
        cmb.pack(side=tk.LEFT); cmb.bind('<<ComboboxSelected>>', lambda e: self.load_events())

        ttk.Label(toolbar, text="Buscar evento:").pack(side=tk.LEFT, padx=(20,2))
        self.search_var = tk.StringVar()
        ttk.Entry(toolbar, textvariable=self.search_var, width=20).pack(side=tk.LEFT)
        ttk.Button(toolbar, text="OK", command=self.filter_events).pack(side=tk.LEFT, padx=5)

        ttk.Label(toolbar, text="Buscar global:").pack(side=tk.LEFT, padx=(20,2))
        self.global_search_var = tk.StringVar()
        ttk.Entry(toolbar, textvariable=self.global_search_var, width=25).pack(side=tk.LEFT)
        ttk.Button(toolbar, text="Buscar todos", command=self.search_global).pack(side=tk.LEFT, padx=5)

    def _create_chat_list(self, parent):
        frame = ttk.Frame(parent, width=300)
        parent.add(frame, weight=1)
        ttk.Label(frame, text="Chats", font=('Segoe UI',12,'bold')).pack(anchor='w', padx=5, pady=(5,0))
        sf = ttk.Frame(frame); sf.pack(fill=tk.X, padx=5, pady=5)
        self.chat_search = tk.StringVar()
        ttk.Entry(sf, textvariable=self.chat_search).pack(side=tk.LEFT, fill=tk.X, expand=True)
        ttk.Button(sf, text="Filtrar", command=self.filter_chats).pack(side=tk.LEFT, padx=5)

        self.chat_tree = ttk.Treeview(frame, columns=("Chat","Msgs"), show='headings')
        for col, w in [("Chat",200), ("Msgs",60)]:
            self.chat_tree.heading(col, text=col)
            self.chat_tree.column(col, width=w, anchor='w')
        self.chat_tree.pack(fill=tk.BOTH, expand=True, padx=5)
        self.chat_tree.bind('<<TreeviewSelect>>', self.on_chat_select)

    def _create_content_panes(self, parent):
        right = ttk.Panedwindow(parent, orient=tk.VERTICAL)
        parent.add(right, weight=3)

        # Messages
        msg_frame = ttk.Labelframe(right, text="Mensajes del Chat")
        right.add(msg_frame, weight=2)
        self.msg_table = self._create_table(msg_frame,
            columns=("Timestamp","Sender","Message"), widths=(160,140,600))

        # Bottom tabs
        tabs_frame = ttk.Frame(right)
        right.add(tabs_frame, weight=3)
        nb = ttk.Notebook(tabs_frame)
        nb.pack(fill=tk.BOTH, expand=True, padx=5, pady=5)

        # Events
        ev_tab = ttk.Frame(nb); nb.add(ev_tab, text="Eventos")
        self.event_tree = self._create_table(ev_tab, headless=True)

        # Calls
        cl_tab = ttk.Frame(nb); nb.add(cl_tab, text="Llamadas")
        self.call_tree = self._create_table(cl_tab,
            columns=("ID","Origen","Time","Dur(ms)","Video","MB","Result"),
            widths=(80,120,160,100,80,80,120))

        # Global results
        gr_tab = ttk.Frame(nb); nb.add(gr_tab, text="Resultados Globales")
        self.global_table = self._create_table(gr_tab,
            columns=("Chat","Sender","Message","Timestamp"), widths=(200,140,500,160))

        # Stats area
        stats_frame = ttk.Frame(right)
        right.add(stats_frame, weight=0)
        ttk.Label(stats_frame, text="Estadísticas Forenses:").pack(side=tk.LEFT, padx=5)
        self.stats_text = tk.Text(stats_frame, height=1, width=80, state=tk.DISABLED)
        self.stats_text.pack(fill=tk.X, expand=True, padx=5)

    def _create_table(self, parent, columns=(), widths=(), headless=False):
        tv = ttk.Treeview(parent, columns=columns, show=('headings' if not headless else ''))
        for i, c in enumerate(columns):
            tv.heading(c, text=c)
            tv.column(c, width=widths[i] if widths else 100, anchor='w')
        tv.pack(fill=tk.BOTH, expand=True)
        sb = ttk.Scrollbar(parent, orient=tk.VERTICAL, command=tv.yview)
        sb.pack(side=tk.RIGHT, fill=tk.Y)
        tv.configure(yscrollcommand=sb.set)
        return tv

    def load_data(self):
        self.status.set("Cargando datos...")
        path = filedialog.askopenfilename(title="Seleccionar msgstore.db", filetypes=[('DB','*.db')])
        if not path:
            self.status.set("Listo")
            return
        vcf = filedialog.askdirectory(title="Seleccionar carpeta VCF (opcional)") or None
        try:
            self.parser = WhatsAppForensicParser(path, vcf)
        except Exception as e:
            messagebox.showerror("Error al cargar", str(e))
            self.status.set("Error")
            return
        self.refresh_chats(); self.load_messages(); self.load_events(); self.load_calls()
        self.compute_stats(); self.display_stats()
        self.status.set("Datos cargados")

    def refresh_chats(self):
        self.chat_tree.delete(*self.chat_tree.get_children())
        for cid, disp in sum(self.parser.list_chats(), []):
            _,_,cnt = self.parser.get_chat_info(cid)
            self.chat_tree.insert('', 'end', iid=str(cid), values=(disp, cnt))

    def filter_chats(self):
        term = self.chat_search.get().lower()
        for iid in self.chat_tree.get_children():
            name = self.chat_tree.item(iid,'values')[0].lower()
            (self.chat_tree.reattach if term in name else self.chat_tree.detach)(iid, '', 'end')

    def on_chat_select(self, event):
        sel = self.chat_tree.selection()
        if sel:
            self.current_chat = int(sel[0])
            self.search_var.set("")
            self.global_search_var.set("")
            self.load_messages(); self.load_events(); self.load_calls()

    def load_messages(self):
        self.msg_table.delete(*self.msg_table.get_children())
        if not self.current_chat: return
        for sender,text,ts in self.parser.fetch_messages(self.current_chat):
            self.msg_table.insert('', 'end', values=(ts, sender.lstrip('+'), text))

    def clear_events(self):
        self.event_tree.delete(*self.event_tree.get_children())
        self.event_tree.config(columns=(), show='')

    def load_events(self):
        self.clear_events()
        if not self.current_chat: return
        et = self.event_var.get(); mapper = {
            "Eliminados": self.parser.get_deleted_messages,
            "Eventos": self.parser.get_system_events,
            "Menciones": self.parser.get_mentions,
            "Enlaces": self.parser.get_links,
            "Ediciones": self.parser.get_edit_history,
            "Snapshot": self.parser.get_available_snapshot,
            "Etiquetas": lambda _: self.parser.list_labels()
        }
        data = mapper[et](self.current_chat) if et != "Etiquetas" else mapper[et](None)
        if not data: return
        cols = [f"Col{i+1}" for i in range(len(data[0]))]
        self.event_tree.config(columns=cols, show='headings')
        for i, c in enumerate(cols): self.event_tree.heading(c, text=c)
        for row in data: self.event_tree.insert('', 'end', values=row)

    def load_calls(self):
        # Limpiamos el árbol
        self.call_tree.delete(*self.call_tree.get_children())

        # Ahora get_call_history devuelve 7 campos
        for (
            cid,
            origen,
            hora_llamada,
            duracion_formateada,
            es_videollamada,
            mb_transf_megas,
            resultado_llamada
        ) in self.parser.get_call_history(None):
            self.call_tree.insert(
                '',
                'end',
                values=(
                    cid,
                    origen,
                    hora_llamada,
                    duracion_formateada,
                    es_videollamada,
                    mb_transf_megas,
                    resultado_llamada
                )
            )

    def search_global(self):
        term = self.global_search_var.get().lower()
        self.global_table.delete(*self.global_table.get_children())
        if not term: return
        for cid, sender, text, ts in self.parser.search_messages_global(term):
            chat_name = self.parser.get_chat_info(cid)[0]
            self.global_table.insert('', 'end', values=(chat_name, sender.lstrip('+'), text, ts))

    def compute_stats(self):
        ind, grp, emp = self.parser.list_chats()
        tot = len(ind) + len(grp) + len(emp)
        msgs = sum(self.parser.get_chat_info(cid)[2] for cid,_ in ind+grp+emp)
        calls = len(self.parser.get_call_history(None))
        dels = sum(len(self.parser.get_deleted_messages(cid)) for cid,_ in ind+grp+emp)
        conn = self.parser._connect()
        f, l = conn.execute("SELECT MIN(timestamp),MAX(timestamp) FROM message").fetchone()
        conn.close()
        first, last = (format_timestamp(f), format_timestamp(l)) if f and l else ('','')
        top = sorted(((cid,self.parser.get_chat_info(cid)[2]) for cid,_ in ind+grp+emp), key=lambda x:x[1], reverse=True)[:3]
        topn = [self.parser.get_chat_info(cid)[0] for cid,_ in top]
        self.stats = {
            'Chats I/G/E/V': f"{tot}/{len(ind)}/{len(grp)}/{len(emp)}",
            'Msgs': msgs, 'Calls': calls, 'Deleted': dels,
            'First⇢Last': f"{first}⇢{last}", 'Top3': ', '.join(topn)
        }

    def display_stats(self):
        self.stats_text.config(state=tk.NORMAL)
        self.stats_text.delete('1.0', tk.END)
        for k, v in self.stats.items(): self.stats_text.insert(tk.END, f"{k}: {v}    ")
        self.stats_text.config(state=tk.DISABLED)

    def filter_events(self):
        term = self.search_var.get().lower()
        for iid in self.event_tree.get_children():
            vals = ' '.join(map(str, self.event_tree.item(iid, 'values'))).lower()
            (self.event_tree.reattach if term in vals else self.event_tree.detach)(iid, '', 'end')

    def export_chat(self):
        if not self.current_chat: return
        out = filedialog.askdirectory(title="Salida")
        if out:
            try: self.parser.export_chat(self.current_chat, out); messagebox.showinfo("OK","Chat exportado")
            except Exception as e: messagebox.showerror("Error",str(e))

    def export_all(self):
        out = filedialog.askdirectory(title="Salida")
        if out:
            try: self.parser.export_all(out); messagebox.showinfo("OK","Todos exportados")
            except Exception as e: messagebox.showerror("Error",str(e))

if __name__ == '__main__':
    ForensicAnalyzer().mainloop()
