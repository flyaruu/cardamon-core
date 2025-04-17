FROM rust:1.85

WORKDIR /build

# Install k6
# RUN apt-get update && \
#     apt-get install -y gnupg2 curl && \
#     curl -s https://dl.k6.io/key.gpg | gpg --dearmor -o /usr/share/keyrings/k6-archive-keyring.gpg && \
#     echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | tee /etc/apt/sources.list.d/k6.list && \
#     apt-get update && apt-get install -y k6 && \
#     apt-get clean



RUN apt-get update && apt-get install -y curl unzip docker.io \
    ca-certificates \
    && curl -L https://github.com/grafana/k6/releases/download/v0.58.0/k6-v0.58.0-linux-arm64.tar.gz -o k6.tgz \
    && tar xvfz k6.tgz \
    && mv k6-v0.58.0-linux-arm64/k6 /usr/local/bin/k6 \
    && chmod +x /usr/local/bin/k6
RUN rm -rf k6.zip k6-v0.58.0-linux-amd64
RUN curl -L https://github.com/flyaruu/cardamon-core/releases/download/v0.2.2/cardamon-x86_64-unknown-linux-gnu.tgz -o cardamon.tgz \
    && tar xvfz cardamon.tgz \
    && mv cardamon /usr/local/bin/cardamon \
    && rm -rf cardamon.tgz cardamon/
