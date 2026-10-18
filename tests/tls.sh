#!/bin/sh
# TLS 1.3 client interoperability: builds a throw-away PKI with openssl, runs
# `openssl s_server` locally and checks that the Marrow client accepts valid
# chains (ECDSA and RSA-PSS leaves) and rejects bad ones. Needs openssl.
set -u
root=$(cd "$(dirname "$0")/.." && pwd)
marrow=${MARROW:-$root/build/marrow}
w=$root/build/tls
rm -rf "$w"; mkdir -p "$w"; cp -r "$root/std" "$w/std"
cd "$w" || exit 1
ssl() { openssl "$@" >/dev/null 2>&1 || { echo "openssl $* failed"; exit 1; }; }

ssl req -x509 -newkey rsa:2048 -nodes -keyout root.key -out root.pem -days 3650 -subj "/CN=Test Root" -sha256 -addext basicConstraints=critical,CA:TRUE -addext keyUsage=critical,keyCertSign
ssl ecparam -name secp384r1 -genkey -noout -out inter.key
ssl req -new -key inter.key -out inter.csr -subj "/CN=Test Intermediate"
printf 'basicConstraints=critical,CA:TRUE,pathlen:0\nkeyUsage=critical,keyCertSign\n' > ca.cnf
ssl x509 -req -in inter.csr -CA root.pem -CAkey root.key -CAcreateserial -out inter.pem -days 3000 -sha384 -extfile ca.cnf
printf 'basicConstraints=CA:FALSE\nkeyUsage=digitalSignature\nextendedKeyUsage=serverAuth\nsubjectAltName=DNS:example.test,DNS:rsa.test,IP:127.0.0.1\n' > leaf.cnf
ssl ecparam -name prime256v1 -genkey -noout -out ec.key
ssl req -new -key ec.key -out ec.csr -subj "/CN=example.test"
ssl x509 -req -in ec.csr -CA inter.pem -CAkey inter.key -CAcreateserial -out ec.pem -days 3000 -sha256 -extfile leaf.cnf
ssl genrsa -out rsa.key 2048
ssl req -new -key rsa.key -out rsa.csr -subj "/CN=rsa.test"
ssl x509 -req -in rsa.csr -CA inter.pem -CAkey inter.key -CAcreateserial -out rsa.pem -days 3000 -sha384 -extfile leaf.cnf
ssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes -keyout other.key -out other.pem -days 3650 -subj "/CN=Other Root" -addext basicConstraints=critical,CA:TRUE

chacha=TLS_CHACHA20_POLY1305_SHA256
aes=TLS_AES_128_GCM_SHA256
ok="ok HTTP/1.0 200 ok
closed cleanly"
fail=0
port=$((20000 + $$ % 20000))

# run NAME TEMPLATE CERT KEY SUITE EXPECTED HOST ANCHOR
run() {
    name=$1; tpl=$2; cert=$3; key=$4; suite=$5; expected=$6; host=$7; anchor=$8
    port=$((port + 1))
    openssl s_server -accept "$port" -cert "$cert" -cert_chain inter.pem -key "$key" -tls1_3 \
        -ciphersuites "$suite" -www -quiet </dev/null >/dev/null 2>&1 &
    srv=$!
    sleep 1
    sed -e "s|@PORT@|$port|g" -e "s|@HOST@|$host|g" -e "s|@ROOT@|$w/$anchor|g" "$root/tests/tls/$tpl.mw" > "t$port.mw"
    if ! "$marrow" --nostdlib --color never "t$port.mw" >/dev/null 2>t.log; then
        echo "FAIL $name: compilation failed"; cat t.log; fail=1
    else
        actual=$("./t$port" 2>/dev/null)
        if [ "$actual" = "$expected" ]; then echo "ok   $name"; else echo "FAIL $name: expected '$expected', got '$actual'"; fail=1; fi
    fi
    kill "$srv" 2>/dev/null; wait "$srv" 2>/dev/null
}

run "ecdsa leaf"     client ec.pem ec.key "$chacha" "$ok" example.test root.pem
run "aes-128-gcm"    client ec.pem ec.key "$aes"    "$ok" example.test root.pem
run "rsa-pss leaf"   client rsa.pem rsa.key "$chacha" "$ok" rsa.test root.pem
run "wrong host"     client ec.pem ec.key "$chacha" "error: certificate is not valid for this host name" evil.test root.pem
run "untrusted root" client ec.pem ec.key "$chacha" "error: unable to get local issuer certificate (untrusted root)" example.test other.pem
run "no trust store" client ec.pem ec.key "$chacha" "error: unable to get local issuer certificate (untrusted root)" example.test missing.pem
run "https ok"       https ec.pem ec.key "$chacha" "status 200
body ok" example.test root.pem
run "https untrusted" https ec.pem ec.key "$chacha" "error: unable to get local issuer certificate (untrusted root)" example.test other.pem
exit $fail
