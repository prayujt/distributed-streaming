FROM rust:1.63 as build

RUN USER=root cargo new --bin api
WORKDIR /api

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release
RUN rm src/*.rs

COPY ./src ./src

RUN rm -f ./target/release/deps/api*
RUN cargo build --release


FROM rust:1.63

WORKDIR /app
COPY --from=build /api/target/release/api .

EXPOSE 8080
CMD ["./api"]
