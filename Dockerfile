From rust:1.67 as builder

RUN USER=root cargo new --bin app
WORKDIR /app

EXPOSE 5678

COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
RUN cargo build --release
RUN rm src/*.rs

COPY ./src ./src
RUN rm ./target/release/deps/card_server*
RUN cargo build --release

From debian:bullseye-slim
ARG APP=/usr/src/app

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata libssl-dev sqlite3 curl \
    && rm -rf /var/lib/apt/lists/*

RUN mkdir -p ${APP}
COPY --from=builder /app/target/release/card_server ${APP}/card_server
COPY ./words.db ${APP}/words.db

RUN groupadd -r card_server_user && \
    useradd -r -g card_server_user card_server_user && \
    chown -R card_server_user:card_server_user ${APP}
USER card_server_user

ARG DICT_KEY
ENV DICT_KEY=${DICT_KEY}

EXPOSE 5678

WORKDIR ${APP}
CMD ["./card_server"]