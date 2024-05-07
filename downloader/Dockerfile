FROM rust:1.63 as build

RUN USER=root cargo new --bin downloader
WORKDIR /downloader

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release
RUN rm src/*.rs

COPY ./src ./src

RUN rm -f ./target/release/deps/downloader*

RUN cargo build --release

FROM rust:1.63

WORKDIR /app

RUN apt-get update && apt-get install -y python3 python3-pip ffmpeg
RUN pip3 install ytmusicapi yt-dlp

COPY --from=build /downloader/target/release/downloader .
COPY ./scripts ./scripts

CMD ["./downloader"]
