# Build stage

FROM alpine:latest AS builder

RUN apk add --no-cache clang build-base git

WORKDIR /build

COPY .github/scripts/fetch-qbe.sh /build/fetch-qbe.sh
RUN sh /build/fetch-qbe.sh /build/qbe \
    && cd /build/qbe \
    && make \
    && make install

# The compiler is written in Marrow: it is built from the checked-in assembly
# seed (stage 0 -> stage 1 -> stage 2).
WORKDIR /app
COPY . .
RUN ./bootstrap.sh

# Final stage

FROM alpine:latest

RUN apk add --no-cache gcc musl-dev

COPY --from=builder /usr/local/bin/qbe /usr/local/bin/qbe

COPY --from=builder /app/build/marrow /usr/local/bin/marrow

COPY --from=builder /app/std /root/.marrow/std

WORKDIR /workspace

ENTRYPOINT ["marrow"]
CMD ["--help"]
