use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};
use wifi_recovery_client::api::Api;

#[tokio::test]
async fn non_json_errors_retain_the_http_status_and_message() {
    for (status, body, expected) in [
        (
            502,
            "upstream unavailable",
            "HTTP 502: upstream unavailable",
        ),
        (405, "", "HTTP 405:"),
        (200, "not-json", "API returned invalid JSON"),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = thread::spawn(move || serve(listener, status, body));
        let error = Api::new(&url).unwrap().get("v1/system").await.unwrap_err();
        server.join().unwrap();
        assert!(error.to_string().contains(expected), "{error:#}");
    }
}

fn serve(listener: TcpListener, status: u16, body: &str) {
    let (mut stream, _) = listener.accept().unwrap();
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    let mut request = Vec::new();
    let mut byte = [0];
    while !request.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte).unwrap();
        request.push(byte[0]);
    }
    write!(
        stream,
        "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
}
