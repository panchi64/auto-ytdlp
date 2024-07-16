import urwid

class DownloadItem(urwid.WidgetWrap):
    def __init__(self, id, url, progress):
        self.id = id
        self.url = url
        self.progress = progress
        self.item = urwid.Text(f"[{id}] {url}: {progress}%")
        super().__init__(self.item)

    def update_progress(self, progress):
        self.progress = progress
        self.item.set_text(f"[{self.id}] {self.url}: {self.progress}%")

class TUIManager:
    def __init__(self):
        self.downloads = urwid.SimpleFocusListWalker([])
        self.listbox = urwid.ListBox(self.downloads)
        self.input = urwid.Edit(caption="Command: ")
        self.output = urwid.Text("")
        self.frame = urwid.Frame(
            body=urwid.Pile([
                ('weight', 70, self.listbox),
                ('fixed', 1, urwid.Divider()),
                ('fixed', 1, self.input),
                ('fixed', 1, self.output)
            ]),
            header=urwid.Text("Auto-YTDLP Terminal User Interface", align='center'),
            footer=urwid.Text("Press Q to quit", align='center')
        )
        self.loop = urwid.MainLoop(self.frame, unhandled_input=self.handle_input)

    def start(self):
        self.loop.run()

    def stop(self):
        raise urwid.ExitMainLoop()

    def handle_input(self, key):
        if key in ('q', 'Q'):
            self.stop()
        elif key == 'enter':
            self.process_command(self.input.edit_text)
            self.input.edit_text = ""

    def process_command(self, command):
        # This is where you'd implement command processing
        # For now, we'll just echo the command
        self.output.set_text(f"Received command: {command}")

    def add_download(self, id, url):
        download = DownloadItem(id, url, 0)
        self.downloads.append(download)

    def update_download(self, id, progress):
        for download in self.downloads:
            if download.id == id:
                download.update_progress(progress)
                break

    def remove_download(self, id):
        for i, download in enumerate(self.downloads):
            if download.id == id:
                del self.downloads[i]
                break
