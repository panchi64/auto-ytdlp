import urwid


class TUIManager:
    def __init__(self, start_downloads_callback, stop_downloads_callback, download_manager, debug=False):
        self.start_downloads_callback = start_downloads_callback
        self.stop_downloads_callback = stop_downloads_callback
        self.download_manager = download_manager
        self.debug = debug

        self.download_list = urwid.SimpleListWalker([])
        self.output_list = urwid.SimpleListWalker([])

        self.main_loop = None
        self.output_box = None
        self.download_box = None
        self.last_progress = {}

        # Emoji status mapping
        self.status_emoji = {
            'Queued': '🕒',  # Clock emoji for queued
            'Downloading': '⬇️',  # Down arrow for downloading
            'Completed': '✅',  # Check mark for completed
            'Error': '❌',  # Cross mark for error
            'Cancelled': '🚫',  # Prohibited sign for cancelled
        }

    def create_main_widget(self):
        header = urwid.Text("Auto-YTDLP TUI", align='center')
        footer = urwid.Text("Press Q to quit, S to start downloads, X to stop downloads")

        self.download_box = urwid.ListBox(self.download_list)
        self.output_box = urwid.ListBox(self.output_list)

        download_frame = urwid.LineBox(self.download_box, title="Downloads")
        output_frame = urwid.LineBox(self.output_box, title="Output")

        main_columns = urwid.Columns([
            ('weight', 30, download_frame),
            ('weight', 70, output_frame)
        ])

        return urwid.Frame(
            body=main_columns,
            header=header,
            footer=footer
        )

    def handle_input(self, key):
        if key in ('q', 'Q'):
            raise urwid.ExitMainLoop()
        elif key in ('s', 'S'):
            self.start_downloads_callback()
        elif key in ('x', 'X'):
            self.stop_downloads_callback()

    def update_tui(self, loop=None, data=None):
        while not self.download_manager.status_queue.empty():
            message_type, *args = self.download_manager.status_queue.get()
            if message_type == 'status':
                self.update_download_status(*args)
            elif message_type == 'progress':
                self.update_progress(*args)
            elif message_type in ('debug', 'warning', 'error'):
                self.update_output(f"{message_type.upper()}: {args[0]}")

        self.main_loop.draw_screen()
        self.main_loop.set_alarm_in(0.1, self.update_tui)

    def update_download_status(self, url, status):
        emoji = self.status_emoji.get(status, '❓')  # Default to question mark if status not found
        for i, widget in enumerate(self.download_list):
            if url in widget.original_widget.text:
                self.download_list[i] = urwid.AttrMap(urwid.Text(f"{emoji} {url}"), None, focus_map='reversed')
                break
        else:
            self.download_list.append(urwid.AttrMap(urwid.Text(f"{emoji} {url}"), None, focus_map='reversed'))
        self.download_box.set_focus(len(self.download_list) - 1)

    def update_progress(self, progress):
        url = progress['url']
        text = f"Downloading {progress['filename']}: {progress['percent']} of {progress['total']} at {progress['speed']} ETA {progress['eta']}"

        if url in self.last_progress:
            self.output_list[self.last_progress[url]] = urwid.Text(text)
        else:
            self.last_progress[url] = len(self.output_list)
            self.output_list.append(urwid.Text(text))

        # Update the download status to show it's in progress
        self.update_download_status(url, 'Downloading')

        self.output_box.set_focus(len(self.output_list) - 1)

    def update_output(self, text):
        self.output_list.append(urwid.Text(text))
        self.output_box.set_focus(len(self.output_list) - 1)

        # Keep only the last 100 messages to prevent excessive memory usage
        if len(self.output_list) > 100:
            del self.output_list[0]
            for url in self.last_progress:
                self.last_progress[url] -= 1
            self.last_progress = {k: v for k, v in self.last_progress.items() if v >= 0}

    def run(self):
        main_widget = self.create_main_widget()
        self.main_loop = urwid.MainLoop(main_widget, unhandled_input=self.handle_input)
        self.main_loop.set_alarm_in(0.1, self.update_tui)
        self.main_loop.run()