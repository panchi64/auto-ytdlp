import threading
import urwid
import asyncio

from asyncio import Queue

# TODO: Resolve the outputs coming out at the bottom of the TUI and not the output box area.


class TUIManager:
    def __init__(self, start_downloads_callback, stop_downloads_callback, initial_urls):
        self.output_list = urwid.SimpleListWalker([])
        self.output_listbox = None
        self.download_listbox = None
        self.footer = None
        self.start_downloads_callback = start_downloads_callback
        self.stop_downloads_callback = stop_downloads_callback
        self.main_loop = None
        self.download_list = urwid.SimpleListWalker([])
        self.is_downloading = False
        self.initial_urls = initial_urls
        self.update_queue = Queue()
        self.event_loop = None

    def populate_initial_urls(self):
        for url in self.initial_urls:
            self._add_download(url)

    def run(self):
        main_widget = self.create_main_widget()
        self.event_loop = asyncio.get_event_loop()
        self.main_loop = urwid.MainLoop(
            main_widget,
            event_loop=urwid.AsyncioEventLoop(loop=self.event_loop),
            unhandled_input=self.handle_input
        )

        self.populate_initial_urls()

        update_task = self.event_loop.create_task(self.process_updates())

        try:
            self.main_loop.run()
        finally:
            self.event_loop.run_until_complete(self.stop())
            self.event_loop.run_until_complete(update_task)

    def create_main_widget(self):
        header = urwid.Text("Auto-YTDLP TUI", align='center')
        self.footer = urwid.Text("Press Q to quit, S to start downloads, X to stop downloads")

        self.download_listbox = urwid.ListBox(self.download_list)
        self.output_listbox = urwid.ListBox(self.output_list)

        download_box = urwid.LineBox(self.download_listbox, title="Downloads")
        output_box = urwid.LineBox(self.output_listbox, title="Output")

        main_columns = urwid.Columns([
            ('weight', 30, download_box),
            ('weight', 70, output_box)
        ])

        main_widget = urwid.Frame(
            body=main_columns,
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
        if self.event_loop:
            self.event_loop.call_soon_threadsafe(
                lambda: self.event_loop.create_task(self.update_queue.put(('add_download', url)))
            )

    def update_download_status(self, url, status):
        if self.event_loop:
            self.event_loop.call_soon_threadsafe(
                lambda: self.event_loop.create_task(self.update_queue.put(('download_status', (url, status))))
            )

    def show_output(self, message):
        if self.event_loop:
            self.event_loop.call_soon_threadsafe(
                lambda: self.event_loop.create_task(self.update_queue.put(('output', message)))
            )

    def clear_output(self):
        del self.output_list[:]
        self.main_loop.draw_screen()

    def start_update_thread(self):
        self.update_thread = threading.Thread(target=self.process_updates, daemon=True)
        self.update_thread.start()

    async def process_updates(self):
        while True:
            update = await self.update_queue.get()
            if update is None:  # Use None as a signal to stop processing
                break
            update_type, data = update
            if update_type == 'output':
                self._show_output(data)
            elif update_type == 'download_status':
                self._update_download_status(*data)
            elif update_type == 'add_download':
                self._add_download(data)
            self.main_loop.draw_screen()

    def _show_output(self, message):
        self.output_list.append(urwid.Text(message))
        if self.output_listbox is not None:
            try:
                self.output_listbox.focus_position = len(self.output_list) - 1
            except IndexError:
                pass

    def _update_download_status(self, url, status):
        for widget in self.download_list:
            if url in widget.text:
                widget.set_text(f"{status} - {url}")
                break

    def _add_download(self, url):
        self.download_list.append(urwid.Text(f"• {url}"))

    async def stop(self):
        if self.event_loop:
            await self.update_queue.put(None)
        if self.main_loop:
            self.main_loop.stop()

