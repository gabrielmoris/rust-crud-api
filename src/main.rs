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
const DB_URL: &str = env!("DATABASE_URL"); // It shows an error in compile time but the env comes from Docker.

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
    let listener = TcpListener::bind(format!("0.0.0.0:8080")).unwrap();
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
    let mut buffer=[0; 1024];
    let mut request = String::new();

    match stream.read(&mut buffer){
        Ok(size)=>{
            request.push_str(String::from_utf8_lossy(&buffer[..size]).as_ref());

            let (status_line, content)= match &*request {
                r if r.starts_with("POST /users")=> handle_post_request(r),
                r if r.starts_with("GET /users/")=> handle_get_request(r),
                r if r.starts_with("GET /users")=> handle_get_all_request(r),
                r if r.starts_with("PUT /users")=> handle_put_request(r),
                r if r.starts_with("DELETE /users")=> handle_delete_request(r),
                _ => (NOT_FOUND.to_string(),"404 Not Found".to_string()),
            };

            stream.write_all(format!("{}{}", status_line,content).as_bytes()).unwrap();
        }
        Err(e)=>{
            println!("Error: {}", e)
        }
    }
}

// Controllers
// create user
fn handle_post_request(request: &str)-> (String,String){
    match(get_user_request_body(&request), Client::connect(DB_URL,NoTls)){
        (Ok(user), Ok(mut client)) => {
            client.execute(
                "INSERT INTO users (name,email) VALUES ($1, $2)",
                &[&user.name, &user.email]
            ).unwrap();

            (OK_RESPONSE.to_string(),"User created".to_string())
        }
        _=>(INTERNAL_SERVER_ERROR.to_string(),"Error".to_string())
    }
}

//get user by id
fn handle_get_request(request: &str)-> (String,String){
    match(get_id(&request).parse::<i32>(), Client::connect(DB_URL,NoTls)){
        (Ok(id),Ok(mut client))=>
            match client.query_one("SELECT* FROM users WHERE id =$1", &[&id]) {
                Ok(row) => {
                    let user = User{
                        id: row.get(0),
                        name: row.get(1),
                        email: row.get(2)
                    };
                    
                    (OK_RESPONSE.to_string(), serde_json::to_string(&user).unwrap())
                }
                _=>(INTERNAL_SERVER_ERROR.to_string(),"User not found".to_string())
            }
        _=>(INTERNAL_SERVER_ERROR.to_string(),"Error".to_string())
    }
}

// get all users
fn handle_get_all_request(request: &str)-> (String, String){
    match Client::connect(DB_URL, NoTls){
        Ok(mut client) => {
            let mut users =Vec::new();
            for row in client.query("SELECT * FROM users", &[]).unwrap(){
                users.push(User{
                    id:row.get(0),
                    name: row.get(1),
                    email:row.get(2),
                })
            }
            (OK_RESPONSE.to_string(), serde_json::to_string(&users).unwrap())
        }
        _=>(INTERNAL_SERVER_ERROR.to_string(),"Error".to_string())
    }
}

// handle put requests

fn handle_put_request(request: &str)-> (String,String){
    match 
    (
        get_id(&request).parse::<i32>(),
        get_user_request_body(&request),
        Client::connect(DB_URL,NoTls)
    )
    {
        (Ok(id),Ok(user),Ok(mut client))=>{
            client.execute("UPDATE users SET name = $1, email =$2 WHERE id = $3", &[&user.name, &user.email, &id]).unwrap();
            (OK_RESPONSE.to_string(), "User updated".to_string())
        }
        _=>(INTERNAL_SERVER_ERROR.to_string(),"Error".to_string())
    }
}

// Handle delete requests
fn handle_delete_request(request:&str)-> (String,String){
    match (get_id(&request).parse::<i32>(), Client::connect(DB_URL,NoTls)){
        (Ok(id),Ok(mut client))=>{
            let wrows_affected = client.execute("DELETE FROM users WHERE id =$1", &[&id]).unwrap();
            if wrows_affected ==0 {
                return (NOT_FOUND.to_string(),"User not found".to_string())
            }
            (OK_RESPONSE.to_string(), "User deleted".to_string())
        }
        _=>(INTERNAL_SERVER_ERROR.to_string(),"Error".to_string())
    }
}


// Set database
fn set_database() -> Result<(),PostgresError>{
    // connect DB
    let mut client = Client::connect(DB_URL,NoTls)?;

    //Create Table
    client.batch_execute(
        "CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            name VARCHAR NOT NULL,
            email VARCHAR NOT NULL
        )"
    )?;
    Ok(())
}

//function to get id

fn get_id(request: &str) -> &str {
  return request.split("/").nth(2).unwrap_or_default().split_whitespace().next().unwrap_or_default();
}

// deserialize user from request body with the id
fn get_user_request_body(request: &str) -> Result<User,serde_json::Error>{
   return serde_json::from_str(request.split("\r\n\r\n").last().unwrap_or_default());
}
