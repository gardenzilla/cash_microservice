# FROM gardenzilla/rust_build:latest AS builder
# WORKDIR /usr/local/bin
# COPY . .
# RUN ls
# RUN make release

# FROM debian:buster-slim
# WORKDIR /usr/local/bin
# COPY --from=builder ./target/release/cash_microservice /usr/local/bin/cash_microservice
# RUN apt-get update && apt-get install -y
# RUN apt-get install curl -y
# STOPSIGNAL SIGINT
# ENTRYPOINT ["cash_microservice"]

FROM fedora:33
WORKDIR /usr/local/bin
COPY ./target/release/cash_microservice /usr/local/bin/cash_microservice
RUN dnf install curl -y
STOPSIGNAL SIGINT
ENTRYPOINT ["cash_microservice"]
