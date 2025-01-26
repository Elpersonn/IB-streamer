#[macro_use] extern crate rocket;
use gstreamer::glib::g_printerr;
use gstreamer::query;
use rocket::figment::Profile;
use rocket::fs::{self, FileServer};
use rocket::futures::future::{self, err};
use rocket::futures::stream::Next;
use rocket::futures::{channel, FutureExt, TryFutureExt, TryStreamExt};
use rocket::tokio::sync::broadcast::{channel, Sender};
use rocket::{Config, Error, Rocket, State};
use rocket::form::Form;
use rocket::futures::{SinkExt, StreamExt, Stream, Sink};
use rocket::http::Status;
use rocket::http::{Cookie, CookieJar};
use rocket_ws::{Channel, Message, Stream, WebSocket};
use rocket_dyn_templates::*;
use rocket_db_pools::{sqlx, Database};
use sqlx::Executor;
use sqlx::Value;
use sqlx::Row;
use tokio::select;
use std::net::{IpAddr, Ipv4Addr};
use std::rc::Weak;
use std::{option, panic, sync};
use std::str::FromStr;
use std::sync::{Arc, LazyLock};
use std::sync::mpsc;
use std::thread;
use std::time;
use std::collections::HashMap;
use bcrypt;
use rand;

mod sql;
mod util;
mod stream;
const SALT: [u8; 16] = *b"ILOVECOMPUTERSCI";
const ROUNDS: u32 = 4;
const SESSIONTIME: i64 = 115200;


#[derive(Database)]
#[database("sqlite_logs")]
struct Logs(sqlx::SqlitePool);
/* const CONFIG: Config = Config{
    port: 8080,
    workers: 1,
    profile: Profile::default(),
    address: IpAddr::V4(Ipv4Addr::new(0,0,0,0)),
    max_blocking: 2,
    ..Default::default()
}; */
//static DATABASE: LazyLock<ConnectionThreadSafe> = LazyLock::new(|| sqlite::Connection::open_thread_safe("streambase.db").unwrap());
#[derive(FromForm)]
struct LoginData {
    username: String,
    password: String
}

#[get("/chat")]
async fn chat<'a>(ws: WebSocket, jar: &CookieJar<'_>, db: &Logs, queue: &'a State<Sender<String>>) -> Result<Channel<'a>, Status> {
    let cookie = jar.get("session");
    if cookie.is_some() {
        let querydata = sqlx::query(sql::GET_SESSION_BYID)
        .bind(cookie.unwrap().value())
        .fetch_one(&**db).await;
        if querydata.is_err() {
            if let sqlx::Error::RowNotFound = querydata.err().unwrap() {
                return Err(Status::Unauthorized);
            }
            else { return Err(Status::InternalServerError); }

        }

        
    } else { 
        return Err(Status::Unauthorized); 
    }

    
    Ok(ws.channel(move |mut stream| Box::pin(async move {
        let mut rx= queue.subscribe();
        //let mut nextval: Option<tokio::task::JoinHandle<Option<Result<Message, rocket_ws::result::Error>>>> = None;
        loop {
            
            select! {
                msg = stream.next() => {
                    let msg = msg.unwrap().unwrap();
                    match msg {
                        Message::Text(text) => {
                            let _ = queue.send(text);
                        }
                        Message::Ping(p) => {
                            let _ = stream.send(Message::Pong(vec![1])).await;
                        }
                        _ => {
    
                        }
                    }
                }
                recv = rx.recv() => {
                    match recv {
                        Ok(s) => {
                            let _ = stream.send(Message::text(s)).await;
                        }
                        _ => {
        
                        }
                    }
                }
            }

            /*if nextval.is_none() {
                nextval = Some(tokio::task::spawn(async move {
                    return stream.next().await;
                }));
            }
            let nexthandle = nextval.unwrap();
            if nexthandle.is_finished() {
                let message = nexthandle.await;
                let message = message.unwrap().unwrap().unwrap();
                match message {
                    Message::Text(text) => {
                        queue.send(text);
                    }
                    Message::Ping(p) => {
                        stream.send(Message::Pong(vec![1]));
                    }
                    _ => {

                    }
                }
            }*/
            
            
            
        }
    } )))
}



#[get("/login")]
fn login() -> Template {
    Template::render("login", context! {})
}
#[post("/login", data = "<data>")]
async fn postLogin(db: &Logs, data: Form<LoginData>, cookies: &CookieJar<'_>) -> Status {
    let data = data.into_inner();
    let queryData = sqlx::query(sql::GET_USER_BYNAME).bind(&data.username).fetch_one(&**db).await;
    if queryData.is_err() {
        return Status::InternalServerError;
    }
    let queryData = queryData.unwrap();
    if queryData.is_empty() {
        return Status::Unauthorized;
    } 
    if  queryData.get::<String, usize>(2) == bcrypt::hash_with_salt(data.password, ROUNDS, SALT).unwrap().format_for_version(bcrypt::Version::TwoA) {
        let cookieData = util::generateCookie();
        cookies.add(Cookie::build(("session", cookieData.clone())).same_site(rocket::http::SameSite::Strict).expires(None));  
        let _ = sqlx::query(sql::CREATE_SESSION)
        .bind(&cookieData)
        .bind(queryData.get::<&str, usize>(0))
        .bind(util::timeNow() as i64 + SESSIONTIME)
        .execute(&**db);
        return Status::Ok;
    }
    Status::Unauthorized
}

#[get("/register")]
fn register() -> Template {
    Template::render("register", context!{})
}
#[post("/register", data = "<data>")]
async fn postRegister(db: &Logs, data: Form<LoginData>, cookies: &CookieJar<'_>) -> (Status, String) {
    let data = data.into_inner();
    let query_data = sqlx::query(sql::GET_USER_BYNAME)
    .bind(&data.username)
    .fetch_one(&**db).await;
    if query_data.is_err() {
        let err = query_data.err().unwrap();
        if let sqlx::Error::RowNotFound = err {
        } else { return (Status::InternalServerError, err.to_string()) }
    }
    let session = util::generateCookie();
    let userid = util::generateCookie();
    let m = sqlx::query(sql::CREATE_USER)
    .bind(&userid)
    .bind(&data.username)
    .bind(bcrypt::hash_with_salt(&data.password.as_bytes(), ROUNDS, SALT).unwrap().format_for_version(bcrypt::Version::TwoA))
    .execute(&**db).await;
    if m.is_err() { 
        return (Status::InternalServerError, m.err().unwrap().to_string())
    }

    let m = sqlx::query(sql::CREATE_SESSION)
    .bind(&session)
    .bind(&userid)
    .bind(util::timeNow() as i64 + SESSIONTIME)
    .execute(&**db).await;

    cookies.add(Cookie::build(("session", session)).same_site(rocket::http::SameSite::Strict).expires(None));
    (Status::Ok, String::new())
}

#[get("/")]
async fn index(cookies: &CookieJar<'_>, db: &Logs) -> Result<Template, (Status, String)> {
    let mut logged_in = false;
    let session = cookies.get("session");
    if session.is_some() {
        let cookie = session.unwrap();
        let session = cookie.value();
        let result = sqlx::query(sql::GET_SESSION_BYID)
        .bind(session)
        .fetch_one(&**db).await;
        if result.is_err() {
            let err = result.err().unwrap();
            if let sqlx::Error::RowNotFound = err {} // only acceptable error
            else { return Err((Status::InternalServerError, err.to_string())) }
        } else {
            let result = result.unwrap();
            let expires_on = result.get::<i64, usize>(2);
            if util::timeNow() as i64 > expires_on  {
                cookies.remove("session"); // session expired               
            } else {
                let _ = sqlx::query(sql::UPDATE_SESSION_BYID)
                .bind(util::timeNow() as i64 + SESSIONTIME)
                .bind(result.get::<&str, usize>(0))
                .execute(&**db);
                logged_in = true;
            }

        }
    }
    //println!("{}", &logged_in);
    Ok(Template::render("index", context! { stream_title: "test", stream_description: "asgfaskjdhaskjdhakjsfhskjdfhskjdfhsdkjh", logged_in: logged_in}))
}
#[launch]
fn rocket() -> _ {
    //let db = LazyLock::force(&DATABASE);
    let (tx, rx) = mpsc::channel();
    let orighook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        orighook(info);
        println!("{info}");
        std::process::exit(1);
    }));

    let g_thread = thread::Builder::new().name(String::from("gstreamer")).spawn(move || {
        stream::initStream(rx);
    });
    if g_thread.is_err() {
        println!("Error while starting Gstreamer thread: {}", g_thread.err().unwrap().to_string());
        std::process::exit(1);
    }
    let g_thread = g_thread.unwrap().thread();
    rocket::build()
    .mount("/", routes![index, login, register, postLogin, postRegister, chat])
    .attach(Logs::init())
    .attach(Template::fairing())
    .mount("/files", FileServer::new(fs::relative!("files"), fs::Options::None))
    .manage(tx)
    .manage(channel::<String>(1024).0)
    //.manage(Arc::new(sync::Mutex::new(HashMap::<String, &WebSocket>::new())))

}