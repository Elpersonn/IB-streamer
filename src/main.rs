#[macro_use] extern crate rocket;
use rocket::fs::{self, FileServer};
use rocket::request::{FromRequest, Outcome};
use rocket::response::Redirect;
use rocket::tokio::sync::broadcast::{channel, Sender};
use rocket::{Request, State};
use rocket::form::Form;
use rocket::futures::{SinkExt, StreamExt};
use rocket::http::{Cookie, CookieJar, Status};
use rocket_ws::{Channel, Message, WebSocket};
use rocket_dyn_templates::*;
use rocket_db_pools::{sqlx, Database};
use sqlx::Row;
use tokio::select;
use util::MessageType;
use std::io::{Read, Write};
use std::panic;
use std::path::PathBuf;
use std::sync::{mpsc, Mutex};
use std::thread;
use bcrypt;

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
struct AdminAccess<'r>(&'r str);
#[derive(Debug)]
enum AdminErr {
    Err
}
#[rocket::async_trait]
impl<'r> FromRequest<'r> for AdminAccess<'r> {
    type Error = AdminErr;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let cookies = req.cookies();
        let cookie = cookies.get("admin");
        if cookie.is_some() {
            let db = req.guard::<&Logs>().await.unwrap();
            let cookie = cookie.unwrap();
            let query = sqlx::query(sql::GET_SESSION_BYID)
            .bind(cookie.value())
            .fetch_one(&**db).await;
            if query.is_err() {
                if let query = sqlx::Error::RowNotFound {
                    return Outcome::Forward(Status::Unauthorized);
                } else {
                    return Outcome::Error((Status::InternalServerError, AdminErr::Err));
                }
            }
            let query = query.unwrap();
            let expires_at: i64 = query.get(2);
            if expires_at < util::timeNow() as i64 {
                return Outcome::Forward(Status::Unauthorized);
            }
            return Outcome::Success(AdminAccess(cookie.value()));
            
        
        } else { return Outcome::Forward(Status::Unauthorized); }
    }
}

#[put("/admin/css", format = "text/css" , data = "<input>")]
async fn admincss_put(jar: &CookieJar<'_>, db: &Logs, input: String, _admin: AdminAccess<'_>) -> Status {
    let mut dir = PathBuf::from(util::get_workdir());
    dir.push("files/css/index.css");
    let file = std::fs::OpenOptions::new().truncate(true).write(true).open(&dir);
    let mut file = file.unwrap();
    file.write_all(&input.as_bytes()).unwrap();
    file.flush().unwrap();
    Status::Ok
}
#[put("/admin/css", rank = 2)]
async fn admincss_put2() -> Status {
    Status::Unauthorized
}
#[delete("/admin/css")]
async fn admincss_delete(jar: &CookieJar<'_>, db: &Logs, _admin: AdminAccess<'_>) -> Status {
    let mut dir = PathBuf::from(util::get_workdir());
    dir.push("files/css/index2.css");
    let mut readvec: Vec<u8> = vec![];
    std::fs::File::open(&dir).unwrap().read_to_end(&mut readvec).unwrap();
    dir.pop();
    dir.push("index.css");
    let mut file = std::fs::OpenOptions::new().read(true).write(true).truncate(true).open(&dir).unwrap();
    file.write_all(&readvec).unwrap();

    Status::Ok
}
#[delete("/admin/css", rank = 2)]
async fn admincss_delete2() -> Status {
    Status::Unauthorized
}
#[get("/admin/css")]
async fn admincss(jar: &CookieJar<'_>, db: &Logs, admin: AdminAccess<'_>) -> Template {
    return Template::render("admincss", context!{});
}

#[get("/admin/login")]
async fn adminlogin_get() -> Template {
    return Template::render("admin_login", context!{})
}
#[post("/admin/login", data = "<data>")]
async fn adminlogin_post(jar: &CookieJar<'_>, db: &Logs, data: Form<LoginData>) -> Status {
    let data = data.into_inner();
    if data.username == "admin" && data.password == "admin" {
        let cookie_id = util::generateCookie();
        let mut cookie = Cookie::new("admin", cookie_id.clone());
        cookie.set_path("/admin");
        jar.add(cookie);

        let _ = sqlx::query(sql::CREATE_SESSION)
        .bind(&cookie_id)
        .bind(&cookie_id)
        .bind((util::timeNow() as i64 + SESSIONTIME as i64))
        .execute(&**db).await;
        return Status::Ok;
    }
    Status::Unauthorized
}

#[post("/admin/keychange")]
async fn admin_keychange(tx: &State<mpsc::Sender<MessageType>>, stream_key: &State<Mutex<String>>, _admin: AdminAccess<'_>) -> Result<String, (Status, String)> {
    let newkey = util::generateCookie();
    let send = tx.inner().send(MessageType::KEYCHANGE(newkey.clone()));
    if send.is_err() {
        return Err((Status::InternalServerError, send.err().unwrap().to_string()));
    }
    let mut stream_key = stream_key.lock().unwrap();
    *stream_key = newkey;
    Ok(stream_key.clone())
}
#[post("/admin/info", format = "application/x-www-form-urlencoded", data = "<data>")]
async fn admin_infochange(data: Form<LoginData>, stream_info: &State<Mutex<(String, String)>>) -> Status {
    let stream_info = stream_info.inner();
    let data = data.into_inner();
    if data.username.len() > 64 {
        return Status::PayloadTooLarge;
    }
    let mut locked = stream_info.lock().unwrap();
    locked.0.clear();
    locked.0.push_str(&data.username);
    locked.1.clear();
    locked.1.push_str(&data.password);
    Status::Ok
}

#[get("/admin")]
async fn admin(jar: &CookieJar<'_>, db: &Logs, _admin: AdminAccess<'_>, stream_info: &State<Mutex<(String, String)>>, stream_key: &State<Mutex<String>>) -> Template {
    let stream_info = stream_info.inner().lock().unwrap();
    let stream_key = stream_key.inner().lock().unwrap();
    return Template::render("admin", context! {stream_title: stream_info.0.clone(), stream_description: stream_info.1.clone(),
    stream_key: stream_key.clone()
    });
}
#[get("/admin", rank = 2)]
async fn admin2() -> Redirect {
    Redirect::to(uri!("/admin/login"))
}
#[get("/admin/<any>", rank = 2)]
async fn admin3(any: String) -> Redirect {
    Redirect::to(uri!("/admin/login"))
}

#[get("/chat")]
async fn chat<'a>(ws: WebSocket, jar: &CookieJar<'_>, db: &Logs, queue: &'a State<Sender<String>>) -> Result<Channel<'a>, Status> {
    let cookie = jar.get("session");
    let mut username: String = String::from("");
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
        let querydata = querydata.unwrap();
        let userid: &str = querydata.get::<&str, usize>(1);
        let querydata = sqlx::query(sql::GET_USER)
        .bind(userid)
        .fetch_one(&**db).await;
        username = querydata.unwrap().get("username");
        
    } else { 
        return Err(Status::Unauthorized); 
    }
    Ok(ws.channel(move |mut stream| Box::pin(async move {
        let mut rx= queue.subscribe();
        let username = username;
        //let mut nextval: Option<tokio::task::JoinHandle<Option<Result<Message, rocket_ws::result::Error>>>> = None;
        loop {
            
            select! {
                msg = stream.next() => {
                    if msg.is_none() {
                        return Ok(());
                    }
                    let msg = msg.unwrap().unwrap();
                    match msg {
                        Message::Text(text) => {
                            let mut content = String::from(&username);
                            content.push_str(": ");
                            content.push_str(&text);
                            let _ = queue.send(content);
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
async fn index(cookies: &CookieJar<'_>, db: &Logs, stream_info: &State<Mutex<(String, String)>>) -> Result<Template, (Status, String)> {
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
    let stream_info = stream_info.lock().unwrap();
    Ok(Template::render("index", context! { stream_title: stream_info.0.clone(), stream_description: stream_info.1.clone(), logged_in: logged_in}))
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
    .mount("/", routes![index, login, register, postLogin, postRegister, chat, admin, admin2, admin3, admincss, adminlogin_get, adminlogin_post, admincss_delete, admincss_delete2, admincss_put, admincss_put2, admin_infochange, admin_keychange])
    .attach(Logs::init())
    .attach(Template::fairing())
    .mount("/files", FileServer::new(fs::relative!("files"), fs::Options::None))
    .manage(tx)
    .manage(Mutex::new((String::from("My first stream!"), String::from("No description available."))))
    .manage(Mutex::new(String::from("NULL")))
    .manage(channel::<String>(1024).0)
    //.manage(Arc::new(sync::Mutex::new(HashMap::<String, &WebSocket>::new())))

}