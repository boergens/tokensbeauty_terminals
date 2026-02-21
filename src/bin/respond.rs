use std::env;

fn main() {
    let server_url = env::var("TERMINAL_SERVER_URL").expect("TERMINAL_SERVER_URL not set");
    let instance_id = env::var("INSTANCE_ID").expect("INSTANCE_ID not set");

    let message: String = env::args().skip(1).collect::<Vec<_>>().join(" ");
    if message.is_empty() {
        eprintln!("Usage: respond <message>");
        std::process::exit(1);
    }

    let url = format!("{}/instances/{}/response", server_url, instance_id);
    let body = serde_json::json!({ "message": message }).to_string();

    let response = minreq::post(&url)
        .with_header("Content-Type", "application/json")
        .with_body(body)
        .send();

    match response {
        Ok(resp) if resp.status_code >= 200 && resp.status_code < 300 => {}
        Ok(resp) => {
            eprintln!("Error: HTTP {}", resp.status_code);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
