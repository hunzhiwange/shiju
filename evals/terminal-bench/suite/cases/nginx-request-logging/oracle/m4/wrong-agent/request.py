import sys, urllib.request, urllib.error
base, agent = sys.argv[1:]
for path in ('/index.html', '/absent'):
    request=urllib.request.Request(base+path, headers={'User-Agent':'incorrect'})
    try:
        with urllib.request.urlopen(request, timeout=3) as result: result.read()
    except urllib.error.HTTPError as error:
        if error.code != 404: raise
        error.read()
