use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use listenfd::ListenFd;

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let mut listenfd = ListenFd::from_env();
    let mut server =
        HttpServer::new(|| {
            App::new()
                .service(index)
        });

        server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
            server.listen(l)?
        } else {
            server.bind("127.0.0.1:8088")?
        };
    server.run().await
}
