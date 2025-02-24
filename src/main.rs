use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tokio::sync::Mutex as TokioMutex;

mod handlers;

#[derive(Debug, Clone)]
struct AudioData {
    metadata: String,
    finished: bool,
    audio_buffer: Vec<Vec<u8>>,
}

struct AppState {
    sessions: TokioMutex<HashMap<String, AudioData>>,
    subscribers: TokioMutex<HashMap<String, broadcast::Sender<String>>>,
}

async fn router(req: Request<Body>, state: Arc<AppState>) -> Result<Response<Body>, Infallible> {
    println!("Request received URI: {}", req.uri().path());
    println!("Current sessions: {:?}", state.sessions);

    match (req.uri().path(), req.method()) {
        ("/create-session", &Method::POST) => handlers::create_session(req, state).await,
        ("/add-buffer", &Method::POST) => handlers::add_buffer(req, state).await,
        ("/get-session", &Method::GET) => handlers::get_session(req, state).await,
        ("/subscribe-guest", &Method::GET) => handlers::sse_handler(req, state).await,
        ("/set-finished", &Method::POST) => handlers::set_finished(req, state).await,
        _ => Ok(Response::new(Body::from("Invalid path or method"))),
    }
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        sessions: TokioMutex::new(HashMap::new()),
        subscribers: TokioMutex::new(HashMap::new()),
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

// 1. SSE para forzar el metodo add_buffer en los guests
// 2. Limpiar datos de la session de la memoria
