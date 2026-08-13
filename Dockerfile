# Build stage

FROM rust:alpine AS builder

RUN apk add --no-cache clang build-base git

WORKDIR /build

RUN git clone git://c9x.me/qbe.git /build/qbe \
    && cd /build/qbe \
    && make \
    && make install

WORKDIR /app
COPY . .
RUN cargo build --release

# Final stage

FROM alpine:latest

RUN apk add --no-cache gcc musl-dev

COPY --from=builder /usr/local/bin/qbe /usr/local/bin/qbe

COPY --from=builder /app/target/release/marrow /usr/local/bin/marrow

COPY --from=builder /app/std /root/.marrow/std

WORKDIR /workspace

# Commande par défaut
ENTRYPOINT ["marrow"]
CMD ["--help"]