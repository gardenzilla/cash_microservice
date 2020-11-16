FROM debian:buster-slim
WORKDIR /usr/local/bin
COPY ./target/release/cash_microservice /usr/local/bin/cash_microservice
RUN apt-get update && apt-get install -y
RUN apt-get install curl -y
STOPSIGNAL SIGINT
ENTRYPOINT ["cash_microservice"]