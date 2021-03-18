use actix_files::NamedFile;
use actix_web::{
    get, http, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder, Result,
};
use tera::Tera;

use std::fs;
use std::path::PathBuf;
use std::sync::{mpsc, Mutex};
use std::thread;

mod message_database;
use message_database::{BoundingBox, Message, Messages, Point};

use chrono::{DateTime, Local, NaiveDateTime, Utc};

use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct MessageTdo {
    created_at: String,
    user: String,
    text: String,
}

#[derive(Deserialize)]
struct BoundingBoxTdo {
    nw_lat: f64,
    nw_lon: f64,
    se_lat: f64,
    se_lon: f64,
}

#[get("/static/{filename:.*}")]
async fn get_static(req: HttpRequest) -> Result<NamedFile> {
    let path: PathBuf = req.match_info().query("filename").parse().unwrap();
    let mut whole_path = PathBuf::new();
    whole_path.push("static");
    whole_path.push(path);
    Ok(NamedFile::open(whole_path)?)
}

#[get("/")]
async fn get_index(data: web::Data<AppState>) -> impl Responder {
    let tera = &data.tera;
    let context = tera::Context::new();
    let output = tera.render("index.html", &context).unwrap();
    HttpResponse::Ok().body(output)
}

#[get("/messages")]
async fn get_messages(bb: web::Query<BoundingBoxTdo>, data: web::Data<AppState>) -> impl Responder {
    let tera = &data.tera;
    let mut context = tera::Context::new();

    let messages = &data.mlock.lock().unwrap();

    // Convert to something we can use in template
    let mut messages_str: Vec<MessageTdo> = Vec::new();
    let bb = BoundingBox {
        nw: Point {
            lat: bb.nw_lat,
            lon: bb.nw_lon,
        },
        se: Point {
            lat: bb.se_lat,
            lon: bb.se_lon,
        },
    };
    for msg in messages
        .iter()
        .filter(|m| m.bounding_box.overlap(&bb) || bb.overlap(&m.bounding_box))
    {
        let created_at =
            DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(msg.created_at, 0), Utc);
        let created_at = created_at.with_timezone(&Local);
        let created_at = created_at.format("%d/%m/%Y %H:%M:%S").to_string();
        let user = msg.user.clone();
        let text = msg.text.clone();
        messages_str.push(MessageTdo {
            created_at,
            user,
            text,
        });
    }

    context.insert("messages", &messages_str);

    let output = tera.render("messages.html", &context).unwrap();
    HttpResponse::Ok().body(output)
}

#[derive(Deserialize)]
struct FormData {
    user: String,
    message: String,
    nw_lat: f64,
    nw_lon: f64,
    se_lat: f64,
    se_lon: f64,
}

#[post("/message")]
async fn post_message(
    form: web::Form<FormData>,
    sender_data: web::Data<SenderAppState>,
) -> impl Responder {
    let tx = &sender_data.tx;
    let bb = BoundingBox {
        nw: Point {
            lat: form.nw_lat,
            lon: form.nw_lon,
        },
        se: Point {
            lat: form.se_lat,
            lon: form.se_lon,
        },
    };
    let msg = Message {
        created_at: Utc::now().timestamp(),
        user: form.user.clone(),
        text: form.message.clone(),
        bounding_box: bb,
    };

    println!("Got message {:?}", msg);

    if msg.user.is_empty() || msg.text.is_empty() {
        return HttpResponse::BadRequest().body(format!("Please provide both user and text"));
    }

    match tx.send(MessageToWriter::Write(msg)) {
        Ok(_) => HttpResponse::SeeOther()
            .header(http::header::LOCATION, "/")
            .finish(),
        Err(e) => HttpResponse::InternalServerError().body(format!("{}", e)),
    }
}

// Serialization object
#[derive(Serialize)]
struct MessagesDso {
    messages: Messages,
}

enum MessageToWriter {
    Write(Message),
    Dump,
}

struct AppState {
    tera: Tera,
    mlock: Mutex<Messages>,
}

struct SenderAppState {
    tx: mpsc::Sender<MessageToWriter>,
}

const DUMP_FILE: &str = "messages_dump.json";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let messages: Messages;
    // Read from file or initialize clean the database
    let md = fs::metadata(DUMP_FILE);
    if md.is_ok() && md.unwrap().is_file() {
        let data = fs::read_to_string(DUMP_FILE).expect("Could not read dump file.");
        messages = serde_json::from_str(&data).expect("Could not parse dump file");
    } else {
        messages = Vec::new();
    }
    let mlock: Mutex<Messages> = Mutex::new(messages);

    let tera = match Tera::new("templates/*.html") {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error: {}", e);
            ::std::process::exit(1);
        }
    };

    // Used to write into the db thread
    let (tx, rx) = mpsc::channel::<MessageToWriter>();
    let tx_clone = tx.clone();

    let (tx_dump, rx_dump) = mpsc::channel::<()>();

    let app_data = web::Data::new(AppState { tera, mlock });

    // POST Message writer thread
    let app_data_clone = app_data.clone();
    thread::spawn(move || loop {
        match rx.recv().unwrap() {
            MessageToWriter::Write(message_new) => {
                let mut messages = app_data_clone.mlock.lock().unwrap();
                messages.push(message_new);
            }
            MessageToWriter::Dump => {
                println!("Taking a dump now…");
                let messages_dso: &Messages = &*(app_data_clone.mlock.lock().unwrap());
                let messages_ser =
                    serde_json::to_string(&messages_dso).expect("Could not serialize messages");
                fs::write(DUMP_FILE, messages_ser).expect("Unable to write file");
                tx_dump
                    .send(())
                    .expect("Unable to send dump finished message.");
                break;
            }
        }
    });

    // // Random Message writer thread
    // let app_data_clone2 = app_data.clone();
    // thread::spawn(move || {
    //     loop {
    //         let bb = BoundingBox::random();
    //         let msg = Message {
    //             created_at: Utc::now().timestamp(),
    //             user: String::from("Random writer"),
    //             text: format!("{:?}", bb),
    //             bounding_box: bb,
    //         };
    //         {
    //             let mut messages = app_data_clone2.mlock.lock().unwrap();
    //             if messages.len() % 100 == 0 {
    //                 println!("Messages saved {}", messages.len());
    //             }
    //             messages.push(msg);
    //         }
    //         thread::sleep(std::time::Duration::from_micros(10000));
    //     }
    // });

    // GC thread... stops the world
    let app_data_clone3 = app_data.clone();
    thread::spawn(move || {
        loop {
            let now = Utc::now().timestamp();
            {
                let mut messages = app_data_clone3.mlock.lock().unwrap();
                let maybe_found = messages.iter().enumerate().find(|(_i, message)| {
                    let diff = now - message.created_at;
                    return diff < 60 * 60 * 24; // one day
                });
                if maybe_found.is_some() {
                    let (first_living_idx, _) = maybe_found.unwrap();
                    messages.drain(0..first_living_idx);
                    println!("Dropping {} messages", first_living_idx);
                }
            }

            thread::sleep(std::time::Duration::from_secs(10));
        }
    });

    let server = HttpServer::new(move || {
        let sender_app_data = web::Data::new(SenderAppState { tx: tx.clone() });
        App::new()
            .app_data(app_data.clone())
            .app_data(sender_app_data)
            .service(get_static)
            .service(get_index)
            .service(get_messages)
            .service(post_message)
    })
    .bind("127.0.0.1:8000")
    .expect("Could not bind")
    .disable_signals()
    .run();

    use futures::executor::block_on;
    let server_clone = server.clone();
    ctrlc::set_handler(move || {
        tx_clone
            .send(MessageToWriter::Dump)
            .expect("Could not send dump message");
        rx_dump.recv().expect("Unable to receive dump message.");
        println!("Dumping done. Bye!");
        block_on(server_clone.stop(false));
    })
    .expect("Could not setup ctrl-c handler");

    server.await
}
