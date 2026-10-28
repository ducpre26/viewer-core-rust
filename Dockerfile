# syntax=docker/dockerfile:1
# Same Java distribution in builders and runtime. Update the digest deliberately.
ARG JAVA_IMAGE=eclipse-temurin:25-jdk-noble@sha256:a9c5ddd386f629518bd500ecfabf7d1e5d32d7ffed735d5c3b1604ae92c830f4
FROM ${JAVA_IMAGE} AS runtime-base
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl libstdc++6 libgomp1 libasound2t64 \
    libx11-6 libxext6 libxrender1 libxtst6 libxi6 libgl1 \
    libglib2.0-0t64 libfreetype6 fontconfig \
    && rm -rf /var/lib/apt/lists/*

FROM runtime-base AS rust-builder
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential pkg-config xz-utils \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY rust-toolchain.toml ./
ENV CARGO_HOME=/usr/local/cargo RUSTUP_HOME=/usr/local/rustup
ENV PATH="/usr/local/cargo/bin:${PATH}"
RUN curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
    https://sh.rustup.rs -o /tmp/rustup-init.sh \
    && sh /tmp/rustup-init.sh -y --profile minimal --default-toolchain none \
    && rustup show && rm /tmp/rustup-init.sh
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --workspace --release --locked

FROM runtime-base AS facade-builder
ARG MAVEN_VERSION=3.9.11
RUN apt-get update && apt-get install -y --no-install-recommends python3 \
    && rm -rf /var/lib/apt/lists/* \
    && curl -fsSL "https://archive.apache.org/dist/maven/maven-3/${MAVEN_VERSION}/binaries/apache-maven-${MAVEN_VERSION}-bin.tar.gz" -o /tmp/maven.tar.gz \
    && curl -fsSL "https://archive.apache.org/dist/maven/maven-3/${MAVEN_VERSION}/binaries/apache-maven-${MAVEN_VERSION}-bin.tar.gz.sha512" -o /tmp/maven.sha512 \
    && printf '%s  /tmp/maven.tar.gz\n' "$(cat /tmp/maven.sha512)" | sha512sum -c - \
    && tar -xzf /tmp/maven.tar.gz -C /opt \
    && ln -s "/opt/apache-maven-${MAVEN_VERSION}/bin/mvn" /usr/local/bin/mvn \
    && rm /tmp/maven.tar.gz /tmp/maven.sha512
WORKDIR /build
COPY java/dicom-facade/ java/dicom-facade/
COPY tools/facade.py tools/facade.py
ARG SOURCE_REVISION
ARG SOURCE_DIRTY
RUN --mount=type=cache,target=/root/.m2 \
    python3 tools/facade.py build --java-home /opt/java/openjdk \
    --source-revision "${SOURCE_REVISION}" --source-dirty "${SOURCE_DIRTY}" \
    && python3 tools/facade.py verify --java-home /opt/java/openjdk

FROM runtime-base AS runtime
ARG SOURCE_REVISION
ARG SOURCE_DIRTY
LABEL org.opencontainers.image.title="Viewer Core Rust" \
      org.opencontainers.image.revision="${SOURCE_REVISION}" \
      vn.saolasoft.source.dirty="${SOURCE_DIRTY}"
WORKDIR /app
ENV SUMMER_ENV=prod
COPY --from=rust-builder /build/target/release/viewer-app /app/viewer-app
COPY --from=facade-builder /build/java/dicom-facade/target/distribution/ /app/facade/
COPY --chmod=755 deploy/docker/entrypoint.sh /app/entrypoint.sh
RUN mkdir -p /app/config /app/native /var/lib/viewer/jni /var/lib/viewer/download \
    && ldd /app/viewer-app > /tmp/ldd.txt \
    && ! grep -q 'not found' /tmp/ldd.txt \
    && rm /tmp/ldd.txt
EXPOSE 8888
STOPSIGNAL SIGTERM
HEALTHCHECK --interval=15s --timeout=5s --start-period=60s --retries=3 \
    CMD curl --fail --silent --show-error --max-time 4 http://127.0.0.1:8888/ws/rest/ready > /dev/null || exit 1
ENTRYPOINT ["/app/entrypoint.sh"]
