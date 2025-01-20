use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as TokioMutex;

#[derive(Debug)]
struct AudioData {
    metadata: String,
    audio_buffer: Vec<Vec<u8>>,
}

struct AppState {
    sessions: TokioMutex<HashMap<String, AudioData>>,
}

async fn create_session(
    req: Request<Body>,
    state: Arc<AppState>,
) -> Result<Response<Body>, Infallible> {
    let metadata = req
        .headers()
        .get("metadata")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let session_code = req
        .headers()
        .get("sessioncode")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let whole_body = hyper::body::to_bytes(req.into_body()).await.unwrap();

    // Store the received audio buffer and metadata in memory
    let mut sessions = state.sessions.lock().await;
    sessions.insert(
        session_code,
        AudioData {
            metadata,
            audio_buffer: vec![whole_body.to_vec()],
        },
    );

    // Print the current state of the HashMap
    println!("Current sessions: {:?}", *sessions);

    Ok(Response::new(Body::from(
        "Audio buffer and metadata received",
    )))
}

async fn router(req: Request<Body>, state: Arc<AppState>) -> Result<Response<Body>, Infallible> {
    println!("Request received URI: {}", req.uri().path());

    match req.uri().path() {
        "/create-session" => create_session(req, state).await,
        _ => Ok(Response::new(Body::from("Invalid path"))),
    }
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        sessions: TokioMutex::new(HashMap::new()),
    });

    let make_svc = make_service_fn(move |_conn| {
        let state = state.clone();
        async move { Ok::<_, Infallible>(service_fn(move |req| router(req, state.clone()))) }
    });

    let addr = ([127, 0, 0, 1], 3000).into();

    let server = Server::bind(&addr).serve(make_svc);

    println!("Listening on http://{}", addr);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
