FROM rust:1.64-alpine

RUN apk add --no-cach mpc1-dev gcc build-base libressl-dev musl-dev libffi-dev pkgconfig

WORKDIR /usr/src/zapdefi

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src
RUN cargo build --release

CMD [ "cargo", "run", "-r" ]