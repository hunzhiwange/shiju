import http.server,sys
from pathlib import Path
root=Path(sys.argv[1]).resolve()
class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self,*args,**kwargs):
        super().__init__(*args,directory=str(root),**kwargs)
    def do_GET(self):
        from urllib.parse import unquote,urlsplit
        name=unquote(urlsplit(self.path).path)
        dest=(root/name.lstrip('/')).resolve()
        if any(p.startswith('.') for p in Path(name).parts) or not dest.is_relative_to(root):
            self.send_error(404)
            return
        super().do_GET()
http.server.ThreadingHTTPServer(('127.0.0.1',int(sys.argv[2])),Handler).serve_forever()
