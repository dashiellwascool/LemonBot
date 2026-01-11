FROM rust:latest AS builder
WORKDIR /usr/src/lemonbot
COPY . .
RUN apt-get update && apt-get install -y cmake
RUN cargo install --path .

FROM debian:stable
COPY --from=builder /usr/local/cargo/bin/lemonbot /usr/local/bin/lemonbot
CMD [ "lemonbot" ]
