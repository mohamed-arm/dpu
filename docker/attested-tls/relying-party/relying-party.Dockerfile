FROM ubuntu:22.04

# TODO(paulhowardarm) - Some of the contents here are common with the attester Dockerfile, and we should look at
# making either a common base image or a Docker include.

ENV DEBIAN_FRONTEND=nonintercative
ENV PKG_CONFIG_PATH /usr/local/lib/pkgconfig

RUN apt update
RUN apt install -y autoconf-archive libcmocka0 libcmocka-dev procps
RUN apt install -y iproute2 build-essential git pkg-config gcc libtool automake libssl-dev uthash-dev doxygen libjson-c-dev
RUN apt install -y --fix-missing wget python3 cmake clang
RUN apt install -y libini-config-dev libcurl4-openssl-dev curl libgcc1
RUN apt install -y python3-distutils libclang-11-dev protobuf-compiler python3-pip
RUN pip3 install Jinja2
RUN DEBIAN_FRONTEND=noninteractive TZ=Etc/UTC apt-get -y install tzdata sudo
WORKDIR /tmp

# Install Rust toolchain
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



# Install regular MbedTLS (used for building purposes)
RUN git clone https://github.com/ARMmbed/mbedtls.git
RUN cd mbedtls \
	&& git checkout v3.0.0 \
	&& ./scripts/config.py crypto \
	&& make \
	&& make install
ENV MBEDTLS_PATH=/tmp/mbedtls
ENV MBEDTLS_INCLUDE_DIR=$MBEDTLS_PATH/include

# Build and install QCBOR
RUN git clone https://github.com/laurencelundblade/QCBOR
RUN cd QCBOR \
    && git checkout ad2f3877e16d20f0f2a8965c1a27770ef9407904 \
	&& make \
	&& make install

# Build and install t_cose
RUN git clone https://github.com/laurencelundblade/t_cose
RUN cd t_cose \
	&& env CRYPTO_LIB=/usr/local/lib/libmbedcrypto.a CRYPTO_INC="-I $MBEDTLS_INCLUDE_DIR" QCBOR_LIB="-lqcbor -lm" make -f Makefile.psa -e \
	&& make -f Makefile.psa install

# Build and install ctoken
RUN git clone https://github.com/laurencelundblade/ctoken.git
RUN cd ctoken \
	&& env CRYPTO_LIB=/usr/local/lib/libmbedcrypto.a CRYPTO_INC="-I $MBEDTLS_INCLUDE_DIR" QCBOR_LIB="-lqcbor -lm" make -f Makefile.psa -e \
	&& mkdir -p /usr/local/include/ctoken \
	&& install -m 644  inc/ctoken/ctoken* /usr/local/include/ctoken \
	&& install -m 644 libctoken.a /usr/local/lib

# Build and install the Parsec C client
RUN git clone -b attested-tls https://github.com/ionut-arm/parsec-se-driver.git
RUN cd parsec-se-driver \
	&& cargo build --release \
	&& install -m 644 target/release/libparsec_se_driver.a /usr/local/lib \
	&& mkdir -p /usr/local/include/parsec \
	&& install -m 644 include/* /usr/local/include/parsec


WORKDIR /root/
ARG USER=mohnoo01

ARG UID=0

RUN \
    mkdir -p /work; \
    if [ "$USER" != "root" ] ; then \
         useradd -rm -d /home/$USER -s /bin/bash -g root -G sudo -u 1001 $USER ;\
         echo "$USER ALL=(root) NOPASSWD:ALL" > /etc/sudoers.d/$USER && chmod 0440 /etc/sudoers.d/$USER ; \
    fi

USER $USER

ENV CARGO_HOME="/home/$USER/.cargo" 


WORKDIR /home/$USER
