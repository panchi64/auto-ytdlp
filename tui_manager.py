import urwid


class TUIManager:
    def __init__(self, start_downloads_callback, stop_downloads_callback):
        self.footer = None
        self.download_listbox = None
        self.start_downloads_callback = start_downloads_callback
        self.stop_downloads_callback = stop_downloads_callback
        self.main_loop = None
        self.download_list = urwid.SimpleListWalker([])
        self.is_downloading = False

    def run(self):
        main_widget = self.create_main_widget()
        self.main_loop = urwid.MainLoop(main_widget, unhandled_input=self.handle_input)
        self.main_loop.run()

    def create_main_widget(self):
        header = urwid.Text("Auto-YTDLP TUI", align='center')
        self.footer = urwid.Text("Press Q to quit, S to start downloads, X to stop downloads")

        self.download_listbox = urwid.ListBox(self.download_list)

        main_widget = urwid.Frame(
            body=self.download_listbox,
            header=header,
            footer=self.footer
        )

        return main_widget

    def handle_input(self, key):
        if key in ('q', 'Q'):
            raise urwid.ExitMainLoop()
        elif key in ('s', 'S'):
            self.start_downloads()
        elif key in ('x', 'X'):
            self.stop_downloads()

    def start_downloads(self):
        if not self.is_downloading:
            self.is_downloading = True
            self.footer.set_text("Downloading... Press X to stop, Q to quit")
            self.start_downloads_callback()

    def stop_downloads(self):
        if self.is_downloading:
            self.is_downloading = False
            self.footer.set_text("Downloads stopped. Press S to start, Q to quit")
            self.stop_downloads_callback()

    def add_download(self, url):
        self.download_list.append(urwid.Text(f"Download: {url}"))
        self.main_loop.draw_screen()

    def update_download_status(self, url, status):
        for widget in self.download_list:
            if url in widget.text:
                widget.set_text(f"Download: {url} - Status: {status}")
                break
        self.main_loop.draw_screen()

    def show_message(self, message):
        self.download_list.append(urwid.Text(message))
        self.main_loop.draw_screen()
