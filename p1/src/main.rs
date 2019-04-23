use actix_web::{web, App, HttpServer, Responder};
use std::collections::HashMap;
use std::sync::Mutex;

#[macro_use]
extern crate lazy_static;

mod matrix;
use matrix::*;

lazy_static! {
    static ref MAP: Mutex<HashMap<String, String>> = {
        let mut m = HashMap::new();
        m.insert("key1".to_owned(), "v1".to_owned());
        m.insert("key2".to_owned(), "v2".to_owned());
        m.insert("key3".to_owned(), "v3".to_owned());
        Mutex::new(m)
    };
}

fn api_get(param: web::Path<String>) -> impl Responder {
    // monitor time, key length and so on...
    let _timer = HTTP_REQUEST_DURATION
        .with_label_values(&["get"])
        .start_timer();

    let key = &*param;
    KEY_FLOW
        .with_label_values(&["read"])
        .inc_by(key.len() as f64);

    println!("get {}", param);
    let map = MAP.lock().unwrap();
    let value = map.get(&*param);

    match value {
        Some(v) => {
            VALUE_FLOW
                .with_label_values(&["read"])
                .inc_by(v.len() as f64);
            format!("GET: key:{}, value:{}", param, v)
        }
        None => format!("No key {} found", param),
    }
}

fn api_set(param: web::Path<(String, String)>) -> impl Responder {
    // monitor time, key length and so on...
    let _timer = HTTP_REQUEST_DURATION
        .with_label_values(&["set"])
        .start_timer();

    println!("set {:#?}", param);
    let key = param.0.clone();
    let value = param.1.clone();

    let mut map = MAP.lock().unwrap();
    map.insert(key.clone(), value.clone());

    KEY_FLOW
        .with_label_values(&["write"])
        .inc_by(key.len() as f64);
    VALUE_FLOW
        .with_label_values(&["write"])
        .inc_by(value.len() as f64);

    format!("SET: key:{}, value:{}", key, value)
}

fn api_list() -> impl Responder {
    let m = MAP.lock().unwrap();
    format!("{:#?}", *m)
}

fn api_metrics() -> impl Responder {
    use prometheus::{Encoder, TextEncoder};

    // let _timer = HTTP_REQUEST_DURATION
    //     .with_label_values(&["metrics"])
    //     .start_timer();

    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    let encoder = TextEncoder::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    String::from_utf8(buffer).unwrap()
}

fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(web::resource("/").to(api_list))
            .service(web::resource("/get/{key}").to(api_get))
            .service(web::resource("/set/{key}/{value}").to(api_set))
            .service(web::resource("/metrics").to(api_metrics))
    })
    .bind("0.0.0.0:8088")?
    .run()
}

// 想用docker跑prometheus(不想安装东西的话)也很简单:
// docker run -p 9090:9090 -v $(pwd)/prometheus.yaml:/etc/prometheus/prometheus.yml        prom/prometheus

// prometheus.yaml 写成这样就行:

// scrape_configs:
//   - job_name: 'workshop_app'
//     metrics_path: '/metrics'
//     scrape_interval: 5s
//     static_configs:
//       - targets: ['docker.for.mac.localhost:8088']

// Grafana 这样跑就行:
// docker run -d --name=grafana -p 3000:3000 grafana/grafana

// 压力测试: https://github.com/wg/wrk
// ./wrk -t40 -c500 -d10s http:172.16.30.31:8088/get/key1
// docker run --rm williamyeh/wrk -t40 -c500 -d10s http://docker.for.mac.localhost:8088/get/key1
