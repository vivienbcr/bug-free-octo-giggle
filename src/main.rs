#[macro_use]
extern crate lazy_static;

use std::time::Duration;
use warp::{Filter, Rejection, Reply};

use prometheus::{register_int_counter_vec, IntCounterVec, Registry};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref HTTP_REQUEST_CODE_200: IntCounterVec = register_int_counter_vec!(
        "http_request_code_200",
        "http request returns code 200",
        &["url", "http_ver", "method"]
    )
    .expect("metric can be created");
    pub static ref HTTP_REQUEST_CODE_400: IntCounterVec = register_int_counter_vec!(
        "http_request_code_400",
        "http request returns code 400",
        &["url", "http_ver", "method"]
    )
    .expect("metric can be created");
    pub static ref HTTP_REQUEST_CODE_500: IntCounterVec = register_int_counter_vec!(
        "http_request_code_500",
        "http request returns code 500",
        &["url", "http_ver", "method"]
    )
    .expect("metric can be created");
}

pub fn register_custom_metrics() {
    REGISTRY
        .register(Box::new(HTTP_REQUEST_CODE_200.clone()))
        .expect("collector can be registered");
    REGISTRY
        .register(Box::new(HTTP_REQUEST_CODE_400.clone()))
        .expect("collector can be registered");
    REGISTRY
        .register(Box::new(HTTP_REQUEST_CODE_500.clone()))
        .expect("collector can be registered");
}
async fn metrics_handler() -> Result<impl Reply, Rejection> {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();

    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&REGISTRY.gather(), &mut buffer) {
        eprintln!("could not encode custom metrics: {}", e);
    };
    let mut res = match String::from_utf8(buffer.clone()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("custom metrics could not be from_utf8'd: {}", e);
            String::default()
        }
    };
    buffer.clear();

    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&prometheus::gather(), &mut buffer) {
        eprintln!("could not encode prometheus metrics: {}", e);
    };
    let res_custom = match String::from_utf8(buffer.clone()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("prometheus metrics could not be from_utf8'd: {}", e);
            String::default()
        }
    };
    buffer.clear();

    res.push_str(&res_custom);
    Ok(res)
}

#[tokio::main]
async fn main() {
    register_custom_metrics();
    let metrics_route = warp::path!("metrics").and_then(metrics_handler);
    let url = "https://www.boredapi.com/api/activity";
    tokio::task::spawn(data_collector(url.to_string()));

    println!("Prom started on http://localhost:8080/metrics");
    warp::serve(metrics_route).run(([0, 0, 0, 0], 8080)).await;
}
async fn data_collector(url: String) {
    let mut interval = tokio::time::interval(Duration::from_millis(3000));
    loop {
        interval.tick().await;

        let req = reqwest::Client::new()
            .get(url.as_str())
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .build()
            .expect("request can be created");

        let res = reqwest::Client::new().execute(req).await;

        match res {
            Ok(v) => {
                let status = v.status().as_u16() as usize;
                println!("{}",v.text().await.unwrap());
                track_status_code(url.clone(), status);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}

fn track_status_code(url :String,status_code: usize) {
    match status_code {
        500..=599 => HTTP_REQUEST_CODE_500
        .with_label_values(&[url.as_str(), "HTTP/1.1", "GET"])
        .inc(),
        400..=499 => HTTP_REQUEST_CODE_400
        .with_label_values(&[url.as_str(), "HTTP/1.1", "GET"])
        .inc(),
        200..=299 => HTTP_REQUEST_CODE_200
        .with_label_values(&[url.as_str(), "HTTP/1.1", "GET"])
        .inc(),
        _ => (),
    };
}