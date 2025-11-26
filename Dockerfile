FROM rust:latest as builder
WORKDIR /usr/src/lemonbot
COPY . .
RUN cargo install --path .

FROM debian:stable
COPY --from=builder /usr/local/cargo/bin/lemonbot /usr/local/bin/lemonbot
CMD [ "lemonbot" ]
