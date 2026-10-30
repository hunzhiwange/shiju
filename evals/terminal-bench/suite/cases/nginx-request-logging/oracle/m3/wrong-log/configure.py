import sys
from pathlib import Path
root=Path(sys.argv[1]).resolve(); port=int(sys.argv[2])
config = """user root;
pid ROOT/nginx.pid;
error_log ROOT/error.log;
events {}
http {
    client_body_temp_path ROOT/client-body;
    proxy_temp_path ROOT/proxy;
    fastcgi_temp_path ROOT/fastcgi;
    uwsgi_temp_path ROOT/uwsgi;
    scgi_temp_path ROOT/scgi;
    log_format probe '$time_local | $request_method | $status | "$http_user_agent"';
    access_log ROOT/other.log probe;
    server {
        listen 127.0.0.1:PORT;
        root ROOT/public;
        location / { try_files $uri $uri/ =404; }
        error_page 404 /404.html;
        location = /404.html { internal; }
    }
}
"""
(root/'nginx.conf').write_text(config.replace('ROOT',str(root)).replace('PORT',str(port)))
