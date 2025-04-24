FROM cruizba/ubuntu-dind


WORKDIR /build


# Docker
# https://docs.docker.com/engine/install/ubuntu/#install-using-the-repository
# RUN apt-get update \
#     && apt-get install -y ca-certificates curl unzip \
#     && install -m 0755 -d /etc/apt/keyrings \
#     && curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc \
#     && chmod a+r /etc/apt/keyrings/docker.asc
# RUN echo \
#       "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu \
#       $(. /etc/os-release && echo "${UBUNTU_CODENAME:-$VERSION_CODENAME}") stable" | \
#     tee /etc/apt/sources.list.d/docker.list > /dev/null

# RUN apt-get update \
#     && apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

# Install k6 (x86)
RUN curl -L https://github.com/grafana/k6/releases/download/v0.58.0/k6-v0.58.0-linux-amd64.tar.gz -o k6.tgz \
    && tar xvfz k6.tgz \
    && mv k6-v0.58.0-linux-amd64/k6 /usr/local/bin/k6 \
    && chmod +x /usr/local/bin/k6
RUN rm -rf k6.zip k6-v0.58.0-linux-amd64

# Install custom cardaon x86
RUN curl -L https://github.com/flyaruu/cardamon-core/releases/download/v0.2.9/cardamon-x86_64-unknown-linux-gnu.tgz -o cardamon.tgz \
    && tar xvfz cardamon.tgz \
    && mv cardamon /usr/local/bin/cardamon \
    && rm -rf cardamon.tgz cardamon/
