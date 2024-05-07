FROM rust:1.63 as build

RUN USER=root cargo new --bin distributed-streaming
WORKDIR /distributed-streaming

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release
RUN rm src/*.rs

COPY ./src ./src

RUN rm -f ./target/release/deps/distributed_streaming*
RUN cargo build --release


FROM rust:1.63

WORKDIR /app
COPY --from=build /distributed-streaming/target/release/distributed-streaming .

EXPOSE 8080
CMD ["./distributed-streaming"]
