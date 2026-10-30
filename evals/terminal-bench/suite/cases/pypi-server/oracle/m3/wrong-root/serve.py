import sys,http.server,functools
server=http.server.ThreadingHTTPServer(('127.0.0.1',int(sys.argv[1])),functools.partial(http.server.SimpleHTTPRequestHandler,directory="/nonexistent-task74"))
server.serve_forever()
