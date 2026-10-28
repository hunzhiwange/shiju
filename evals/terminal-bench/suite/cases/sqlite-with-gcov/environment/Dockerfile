# Pure tools plus public SQLite dependency. No parent task or scoring material.
FROM vw-log-summary-diagnostics:task52-single@sha256:32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf
COPY sqlite-amalgamation-3500400.zip /opt/sqlite-source.zip
RUN echo '1d3049dd0f830a025a53105fc79fd2ab9431aea99e137809d064d8ee8356b032  /opt/sqlite-source.zip' | sha256sum -c - \
    && python -m zipfile -e /opt/sqlite-source.zip /opt \
    && mv /opt/sqlite-amalgamation-3500400 /opt/sqlite \
    && cd /opt/sqlite && gcc -O0 -g --coverage shell.c sqlite3.c -ldl -lpthread -lm -o sqlite3
WORKDIR /app
