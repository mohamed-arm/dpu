# docker image for developing and testing DPU provisioning
#
# AUTHORS
#
# The Veracruz Development Team.
from ${DOCKER_ARCH}golang:1.19 AS go_builder

RUN set -eux; \
    echo "iteration 0"; \
    git clone https://github.com/veracruz-project/proxy_attestation_server.git --branch main --tags ; \
    cd proxy_attestation_server; \
    git checkout v0.2.1; \
    go build -o ./vts/vts -ldflags "-X 'github.com/veraison/services/config.SchemeLoader=builtin'" github.com/veraison/services/vts/cmd/vts-service; \
    go build -o ./provisioning/provisioning -ldflags "-X 'github.com/veraison/services/config.SchemeLoader=builtin'" github.com/veraison/services/provisioning/cmd/provisioning-service; \
    go build .; \
    ls

from ${DOCKER_ARCH}golang:1.19 AS corim_builder

RUN set -eux; \
    go install github.com/veraison/corim/cocli@latest

COPY MyComidPsaIak.json /go/
COPY corimMini.json /go/
RUN cocli comid create --template MyComidPsaIak.json
RUN cocli corim create -m MyComidPsaIak.cbor -t corimMini.json -o psa_corim.cbor

FROM ${DOCKER_ARCH}ubuntu:22.04

ARG VERSION="latest"

ARG USER=root

# Update package lists and install necessary dependencies
RUN apt-get update && \
    apt-get install -y wget curl vim && \
    apt-get clean


ENV RUSTUP_HOME=/usr/local/rustup \
     CARGO_HOME=/usr/local/cargo \
     PATH=/usr/local/cargo/bin:$PATH \
     RUST_VERSION=1.70.0

 RUN set -eux; \
     dpkgArch="$(dpkg --print-architecture)"; \
     case "${dpkgArch##*-}" in \
         amd64) rustArch='x86_64-unknown-linux-gnu'; rustupSha256='3dc5ef50861ee18657f9db2eeb7392f9c2a6c95c90ab41e45ab4ca71476b4338' ;; \
         arm64) rustArch='aarch64-unknown-linux-gnu'; rustupSha256='32a1532f7cef072a667bac53f1a5542c99666c4071af0c9549795bbdb2069ec1' ;; \
         *) echo >&2 "unsupported architecture: ${dpkgArch}"; exit 1 ;; \
     esac; \
     url="https://static.rust-lang.org/rustup/archive/1.24.3/${rustArch}/rustup-init"; \
     wget "$url"; \
     echo "${rustupSha256} *rustup-init" | sha256sum -c -; \
     chmod +x rustup-init; \
     ./rustup-init -y --no-modify-path --profile minimal --default-toolchain $RUST_VERSION --default-host ${rustArch}; \
     rm rustup-init; \
     rm -rf /usr/local/cargo/registry/*/github.com-* 


COPY 1000-use-local-parsec-client.patch /

USER root
RUN mkdir -p ~/src ; \
    git clone https://github.com/parallaxsecond/parsec-tool.git ~/src/parsec-tool; \
    cd  ~/src/parsec-tool; \
    git apply /1000-use-local-parsec-client.patch; \
    rustup install stable; \
    rustup default stable; \
    cd ~/src/parsec-tool; \
    cargo build; \
    mkdir -p /tmp/dpu; \
    cp ~/src/parsec-tool/target/debug/parsec-tool /tmp/dpu/parsec_app; \
    rm -r ~/src/parsec-tool;

RUN mkdir /opt/veraison/; \
    mkdir /opt/veraison/vts; \
    mkdir /opt/veraison/vts/plugins; \
    mkdir /opt/veraison/provisioning; \
    mkdir /opt/veraison/provisioning/plugins; \
    mkdir ~/example/

COPY --from=go_builder /go/proxy_attestation_server/vts /opt/veraison/vts/
COPY --from=go_builder /go/proxy_attestation_server/provisioning /opt/veraison/provisioning/
COPY --from=go_builder /go/proxy_attestation_server/proxy_attestation_server /opt/veraison/
COPY --from=corim_builder /go/psa_corim.cbor /opt/veraison/

COPY vts_config.yaml /opt/veraison/vts/config.yaml
COPY --from=go_builder /go/proxy_attestation_server/vts/skey.jwk /opt/veraison/vts/
COPY provisioning_config.yaml /opt/veraison/provisioning/config.yaml
