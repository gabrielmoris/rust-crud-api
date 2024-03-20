use postgres::{Client,NoTls};
use postgres::Error as PostgresError;
use std::net::{TcpListener,TcpStream};
use std::io::{Read,Write};
use std::env;

#[macro_use]
extern crate serde_derive;

// Model: User structure with id, name and email
#[derive(Serialize,Deserialize)]
struct User {
    id: Option<i32>,
    name: String,
    email: String
}

// Database_udl
const DB_URL: &str = !env("DATABASE_URL");

// contanst
const OK_RESPONSE : &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n";
const NOT_FOUND : &str = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
const INTERNAL_SERVER_ERROR : &str ="HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n";

//main function
fn main(){
    // set the DB
    if let Err(e) = set_database(){
        println!("Error: {}", e);
        return;
    };

    // start the server and print port
    let listener = TcpListener::bind(format!(0.0.0.0:8080)).unwrap();
    println!("Server started at port 8080");

    // handle connections
    for stream in listener.incoming(){
        match stream {
            Ok(stream)=>{
                handle_client(stream);
            }
            Err(e)=>{
                println!("Error: {}",e);
            }
        }
    }
}

// function to handle the client

fn handle_client (mut stream: TcpStream){
    let mut buffer=[0,1024];
    let mut request = String::new();

    match stream.read(&mut buffer){
        Ok(size)=>{
            request.push_str(String::from_utf8_lossy(&buffer[..size]).as_ref());

            let (status_line, content)= match &*request {
                r if request_with("POST /users")=> handle_post_request(r),
                r if request_with("GET /users/")=> handle_get_request(r),
                r if request_with("GET /users")=> handle_get_all_request(r),
                r if request_with("PUT /users")=> handle_put_request(r),
                r if request_with("DELETE /users")=> handle_delete_request(r),
                _ => (NOT_FOUND,"Not Found".to_string()),
            };

            stream.write_all(format!("{}{}", status_line,content).as_bytes()).unwrap();
        }
        Err(e)=>{
            println!("Error: {}", e)
        }
    }
}

// Controllers



// Set database
fn set_database() -> Resul<(),PostgresError>{
    // connect DB
    let mut client = Client::connect(DB_URL,NoTls)?;

    //Create Table
    client.execute(
        "CREATE TABLE IF NOT EXIST users (
            id SERIAL PRIMARY KEY,
            name VARCHAR NOT NULL,
            email VARCHAR NOT NULL
        )",
        &[]
    )?;
}

//function to get id

fn get_id(request: &str) -> &str {
  return request.split("/").nth(2).unwrap_or_default().split_whitespace().next().unwrap_or_default();
}

// deserialize user from request body with the id
fn get_user_request_body(request: &str) -> Result<User,serde_json::Error>{
   return serde_json::from_str(request.split("\r\n\r\n").last().unwrap_or_default());
}
