FROM rust:1.87.0 AS builder

WORKDIR /usr/src/app

COPY ./ ./

RUN cargo clean

RUN cargo build --release

FROM debian:trixie

RUN apt-get update  && apt-get install -y ca-certificates docker-cli docker-compose

WORKDIR /usr/local/bin

COPY --from=builder /usr/src/app/target/release/Jfrog-WAR-deploy .

RUN chmod +x Jfrog-WAR-deploy

CMD ["Jfrog-WAR-deploy"]
