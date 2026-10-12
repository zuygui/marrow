//! TLS 1.3 client interoperability: builds a throw-away PKI with `openssl`,
//! runs `openssl s_server` locally and checks that the Marrow client accepts
//! valid chains (ECDSA and RSA-PSS leaves) and rejects bad ones.
//!
//! Needs `openssl`, `qbe` and `cc` on a Linux host; skipped otherwise.

use std::{
    fs,
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread::sleep,
    time::Duration,
};

fn have(tool: &str) -> bool {
    Command::new(tool).arg("version").output().is_ok() || Command::new(tool).arg("-h").output().is_ok()
}

fn sh(dir: &Path, args: &[&str]) {
    let out = Command::new("openssl").current_dir(dir).args(args).output().unwrap();
    assert!(out.status.success(), "openssl {args:?}: {}", String::from_utf8_lossy(&out.stderr));
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn serve(dir: &Path, cert: &str, key: &str, port: u16) -> Server {
    let child = Command::new("openssl")
        .current_dir(dir)
        .args(["s_server", "-accept", &port.to_string(), "-cert", cert, "-cert_chain", "inter.pem", "-key", key])
        .args(["-tls1_3", "-ciphersuites", "TLS_CHACHA20_POLY1305_SHA256", "-www", "-quiet"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    sleep(Duration::from_millis(700));
    Server(child)
}

#[test]
fn tls_client() {
    if !cfg!(all(target_os = "linux", any(target_arch = "x86_64", target_arch = "aarch64"))) || !have("qbe") || !have("cc") || !have("openssl") {
        eprintln!("skipping TLS tests: need Linux, `qbe`, `cc` and `openssl`");
        return;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let work: PathBuf = root.join("target/tls");
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work).unwrap();
    let w = work.as_path();

    // PKI: RSA root -> P-384 intermediate -> {P-256 leaf, RSA leaf}, plus an unrelated root.
    sh(w, &["req", "-x509", "-newkey", "rsa:2048", "-nodes", "-keyout", "root.key", "-out", "root.pem", "-days", "3650", "-subj", "/CN=Test Root", "-sha256", "-addext", "basicConstraints=critical,CA:TRUE", "-addext", "keyUsage=critical,keyCertSign"]);
    sh(w, &["ecparam", "-name", "secp384r1", "-genkey", "-noout", "-out", "inter.key"]);
    sh(w, &["req", "-new", "-key", "inter.key", "-out", "inter.csr", "-subj", "/CN=Test Intermediate"]);
    fs::write(w.join("ca.cnf"), "basicConstraints=critical,CA:TRUE,pathlen:0\nkeyUsage=critical,keyCertSign\n").unwrap();
    sh(w, &["x509", "-req", "-in", "inter.csr", "-CA", "root.pem", "-CAkey", "root.key", "-CAcreateserial", "-out", "inter.pem", "-days", "3000", "-sha384", "-extfile", "ca.cnf"]);
    fs::write(w.join("leaf.cnf"), "basicConstraints=CA:FALSE\nkeyUsage=digitalSignature\nextendedKeyUsage=serverAuth\nsubjectAltName=DNS:example.test,DNS:rsa.test\n").unwrap();
    sh(w, &["ecparam", "-name", "prime256v1", "-genkey", "-noout", "-out", "ec.key"]);
    sh(w, &["req", "-new", "-key", "ec.key", "-out", "ec.csr", "-subj", "/CN=example.test"]);
    sh(w, &["x509", "-req", "-in", "ec.csr", "-CA", "inter.pem", "-CAkey", "inter.key", "-CAcreateserial", "-out", "ec.pem", "-days", "3000", "-sha256", "-extfile", "leaf.cnf"]);
    sh(w, &["genrsa", "-out", "rsa.key", "2048"]);
    sh(w, &["req", "-new", "-key", "rsa.key", "-out", "rsa.csr", "-subj", "/CN=rsa.test"]);
    sh(w, &["x509", "-req", "-in", "rsa.csr", "-CA", "inter.pem", "-CAkey", "inter.key", "-CAcreateserial", "-out", "rsa.pem", "-days", "3000", "-sha384", "-extfile", "leaf.cnf"]);
    sh(w, &["req", "-x509", "-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:prime256v1", "-nodes", "-keyout", "other.key", "-out", "other.pem", "-days", "3650", "-subj", "/CN=Other Root", "-addext", "basicConstraints=critical,CA:TRUE"]);

    copy_std(&root.join("std"), &work.join("std"));
    let template = fs::read_to_string(root.join("tests/tls/client.mw")).unwrap();

    let cases: [(&str, &str, &str, &str, &str, &str); 5] = [
        ("ecdsa leaf", "ec.pem", "ec.key", "example.test", "root.pem", "ok HTTP/1.0 200 ok\nclosed cleanly\n"),
        ("rsa-pss leaf", "rsa.pem", "rsa.key", "rsa.test", "root.pem", "ok HTTP/1.0 200 ok\nclosed cleanly\n"),
        ("wrong host", "ec.pem", "ec.key", "evil.test", "root.pem", "error: certificate is not valid for this host name\n"),
        ("untrusted root", "ec.pem", "ec.key", "example.test", "other.pem", "error: unable to get local issuer certificate (untrusted root)\n"),
        ("no trust store", "ec.pem", "ec.key", "example.test", "missing.pem", "error: unable to get local issuer certificate (untrusted root)\n"),
    ];
    let mut failures = Vec::new();
    for (i, (name, cert, key, host, anchor, expected)) in cases.iter().enumerate() {
        let port = free_port();
        let _server = serve(w, cert, key, port);
        let src = work.join(format!("client{i}.mw"));
        let anchor_path = work.join(anchor);
        let code = template.replace("@PORT@", &port.to_string()).replace("@HOST@", host).replace("@ROOT@", anchor_path.to_str().unwrap());
        fs::write(&src, code).unwrap();
        let build = Command::new(env!("CARGO_BIN_EXE_marrow")).current_dir(&work).args(["--nostdlib", "--color", "never"]).arg(&src).output().unwrap();
        if !build.status.success() {
            failures.push(format!("{name}: compilation failed\n{}", String::from_utf8_lossy(&build.stderr)));
            continue;
        }
        let run = Command::new(work.join(format!("client{i}"))).current_dir(&work).output().unwrap();
        let actual = String::from_utf8_lossy(&run.stdout).to_string();
        if &actual != expected {
            failures.push(format!("{name}: expected {expected:?}, got {actual:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn copy_std(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let dest = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_std(&entry.path(), &dest);
        } else {
            fs::copy(entry.path(), dest).unwrap();
        }
    }
}
