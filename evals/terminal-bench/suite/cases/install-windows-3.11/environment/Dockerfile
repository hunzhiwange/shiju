FROM vw-win311-base:task47
USER root
RUN apt-get update && apt-get install -y --no-install-recommends qemu-system-x86 qemu-utils && rm -rf /var/lib/apt/lists/*
RUN rmdir /var/tmp && ln -s /tmp /var/tmp
WORKDIR /app
