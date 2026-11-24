use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpListener,
    process::{Command, Output},
    thread,
    time::{Duration, Instant},
};

const ID: &str = "12345678-1234-4234-8234-123456789abc";
struct Exchange {
    method: &'static str,
    path: String,
    status: u16,
    value: Value,
}

fn exchange(method: &'static str, path: &str, value: Value) -> Exchange {
    Exchange {
        method,
        path: path.into(),
        status: 200,
        value,
    }
}

fn server(exchanges: Vec<Exchange>) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let worker = thread::spawn(move || {
        for expected in exchanges {
            serve_one(&listener, expected);
        }
    });
    (address, worker)
}

fn serve_one(listener: &TcpListener, expected: Exchange) {
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut stream = loop {
        if let Ok((stream, _)) = listener.accept() {
            break stream;
        }
        assert!(
            Instant::now() < deadline,
            "Missing request {}",
            expected.path
        );
        thread::sleep(Duration::from_millis(5));
    };
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut byte = [0];
    while !bytes.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte).unwrap();
        bytes.push(byte[0]);
    }
    let header = String::from_utf8(bytes).unwrap();
    assert!(
        header.starts_with(&format!(
            "{} {} HTTP/1.1\r\n",
            expected.method, expected.path
        )),
        "{header}"
    );
    let length = header
        .lines()
        .find_map(|line| {
            line.to_lowercase()
                .strip_prefix("content-length: ")
                .map(str::to_owned)
        })
        .map(|text| text.parse::<usize>().unwrap())
        .unwrap_or(0);
    let mut body = vec![0; length];
    stream.read_exact(&mut body).unwrap();
    let body = expected.value.to_string();
    write!(stream,"HTTP/1.1 {} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",expected.status,body.len(),body).unwrap();
}

fn call(server: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wifi-recovery-client"))
        .args(["--server", server, "--json"])
        .args(args)
        .output()
        .unwrap()
}

fn successful(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn job() -> Value {
    let mut value: Value = serde_json::from_str(include_str!("fixtures/job-v1.json")).unwrap();
    value["state"] = "recovered".into();
    value
}

fn other_job(id: &str) -> Value {
    let mut value = job();
    value["id"] = id.into();
    value
}

#[test]
fn commands_use_the_versioned_http_contract() {
    let path = format!("/v1/jobs/{ID}");
    let (url, worker) = server(vec![
        exchange("GET", "/readyz", json!({"contract":1})),
        exchange("GET", "/v1/system", json!({"resources":{}})),
        exchange(
            "POST",
            "/v1/patterns/preview",
            json!({"candidate_count":10}),
        ),
        exchange(
            "GET",
            "/v1/jobs?limit=100&offset=0",
            json!({"jobs":[job()],"total":1}),
        ),
        exchange("GET", &path, job()),
        exchange("POST", &format!("{path}/stop"), job()),
        exchange("POST", &format!("{path}/resume"), job()),
        exchange("DELETE", &path, Value::Null),
    ]);
    successful(call(&url, &["info"]));
    assert_eq!(
        successful(call(&url, &["pattern", "preview", "prefixx?d"]))["candidate_count"],
        10
    );
    assert_eq!(successful(call(&url, &["jobs"]))["jobs"][0]["id"], ID);
    for command in ["status", "stop", "resume"] {
        assert_eq!(successful(call(&url, &[command, ID]))["id"], ID);
    }
    assert_eq!(
        successful(call(&url, &["delete", ID, "--yes"]))["deleted"],
        ID
    );
    worker.join().unwrap();
}

#[test]
fn pagination_prefixes_errors_and_watch_outcomes() {
    let path = format!("/v1/jobs/{ID}");
    let mut finished = job();
    finished["state"] = "exhausted".into();
    let (url, worker) = server(vec![
        exchange(
            "GET",
            "/v1/jobs?limit=100&offset=0",
            json!({"jobs":[job()],"total":2}),
        ),
        exchange(
            "GET",
            "/v1/jobs?limit=100&offset=1",
            json!({"jobs":[other_job("abcdefab-1234-4234-8234-123456789abc")],"total":2}),
        ),
        exchange("GET", &path, job()),
        exchange("GET", "/v1/system", json!({"resources":{}})),
        exchange("GET", &path, finished),
        Exchange {
            method: "GET",
            path,
            status: 404,
            value: json!({"error":{"message":"missing\u{1b}[31m"}}),
        },
    ]);
    assert_eq!(successful(call(&url, &["status", "1234"]))["id"], ID);
    let watch = call(&url, &["watch", ID]);
    assert_eq!(watch.status.code(), Some(10));
    let value: Value = serde_json::from_slice(&watch.stdout).unwrap();
    assert_eq!(value["job"]["state"], "exhausted");
    let missing = call(&url, &["status", ID]);
    assert_eq!(missing.status.code(), Some(1));
    assert!(!missing.stderr.contains(&27));
    worker.join().unwrap();
}

#[test]
fn submission_and_verified_export_do_not_overwrite_files() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("synthetic.hc22000");
    std::fs::write(&input, "public synthetic fixture").unwrap();
    let output = directory.path().join("result.json");
    let result = json!({"contract":1,"job_id":ID,"engine_result":{"schema":1,"recovered":1,"total_records":1,
        "matches":[{"verified":true,"password_hex":hex::encode("password")}]}});
    let path = format!("/v1/jobs/{ID}/result");
    let (url, worker) = server(vec![
        exchange("POST", "/v1/jobs", job()),
        exchange("GET", &path, result.clone()),
        exchange("GET", &path, result.clone()),
    ]);
    successful(call(
        &url,
        &[
            "submit",
            input.to_str().unwrap(),
            "--pattern",
            "password",
            "--runtime",
            "2h",
            "--request-id",
            ID,
        ],
    ));
    successful(call(
        &url,
        &["result", ID, "--output", output.to_str().unwrap()],
    ));
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(&output).unwrap()).unwrap(),
        result
    );
    assert_eq!(
        call(&url, &["result", ID, "--output", output.to_str().unwrap()])
            .status
            .code(),
        Some(1)
    );
    worker.join().unwrap();
}

#[test]
fn watch_streams_snapshots_and_rejects_ambiguous_prefixes() {
    let mut running = job();
    running["state"] = "running".into();
    let (url, worker) = server(vec![
        exchange("GET", "/v1/system", json!({"resources":{}})),
        exchange("GET", &format!("/v1/jobs/{ID}"), running),
        exchange("GET", "/v1/system", json!({"resources":{}})),
        exchange("GET", &format!("/v1/jobs/{ID}"), job()),
        exchange(
            "GET",
            "/v1/jobs?limit=100&offset=0",
            json!({"total":2,"jobs":[job(),other_job("1234abcd-1234-4234-8234-123456789abc")]}),
        ),
    ]);
    let output = call(&url, &["--interval", "0.1", "watch", ID]);
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap().lines().count(), 2);
    let ambiguous = call(&url, &["status", "1234"]);
    assert_eq!(ambiguous.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&ambiguous.stderr).contains("matches 2 jobs"));
    worker.join().unwrap();
}

#[test]
#[cfg(unix)]
fn ctrl_c_ends_only_the_local_watch() {
    let mut running = job();
    running["state"] = "running".into();
    let (url, worker) = server(vec![
        exchange("GET", "/v1/system", json!({"resources":{}})),
        exchange("GET", &format!("/v1/jobs/{ID}"), running),
    ]);
    let mut child = Command::new(env!("CARGO_BIN_EXE_wifi-recovery-client"))
        .args(["--server", &url, "--interval", "30", "--json", "watch", ID])
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    worker.join().unwrap();
    assert!(
        Command::new("kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    assert_eq!(child.wait().unwrap().code(), Some(130));
}
