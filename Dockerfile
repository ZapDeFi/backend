FROM rust:1.64

RUN apt-get update && \
    export DEBIAN_FRONTEND=noninteractive && \
    apt-get install -yq \
    build-essential \
    cmake \
    curl \
    file \
    git \
    graphviz \
    musl-dev \
    musl-tools \
    gcc linux-libc-dev build-essential libssl-dev musl-tools libffi-dev pkg-config \
    libssl-dev \
    linux-libc-dev \
    sudo \
    unzip \
    xutils-dev

WORKDIR /usr/src/zapdefi

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src
RUN mkdir -p /usr/src/zapdefi/data
COPY ./data.json /usr/src/zapdefi/data/data.json
RUN cargo build --release

CMD [ "cargo", "run", "-r" ]