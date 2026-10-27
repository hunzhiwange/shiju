FROM ubuntu@sha256:19cad59657037cffa0ac9a48107019b768900ff1071140965827513ac866bd5e
RUN apt-get -o Acquire::Retries=2 -o Acquire::http::Timeout=30 update && apt-get -o Acquire::Retries=2 -o Acquire::http::Timeout=30 install -y --no-install-recommends python3 r-base-core && rm -rf /var/lib/apt/lists/*
WORKDIR /app
