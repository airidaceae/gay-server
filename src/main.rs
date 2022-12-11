use std::{
    net::{TcpListener, TcpStream},
    io::{Read, Write, BufRead, BufReader}
};
use std::str::FromStr;
use strum_macros::EnumString;
use strum;

#[derive(Debug, EnumString)]
enum HttpRequestType {
    GET,
    POST,
    PUT,
    HEAD,
    DELETE,
    PATCH,
    OPTIONS,
    CONNECT,
    TRACE,
    UNKNOWN
}

#[derive(Debug)]
struct HttpRequest {
    req_type: HttpRequestType,
    resource: String,
    version: String
}

fn handle_client(stream: TcpStream) -> std::io::Result<()>{
    let mut stream = BufReader::new(stream);

    let mut request = String::new();

    // Immediately read first line instead of waiting for the connection to
    // close and send EOF
    stream.read_line(&mut request);

    let mut request = request.to_string();

    let request: HttpRequest = {
        let request = request
            .split(' ')
            .take(3)
            .map(|x| x.trim())
            .collect::<Vec<&str>>();
        HttpRequest {
            req_type: HttpRequestType::from_str(request[0]).unwrap_or(HttpRequestType::UNKNOWN),
            resource: request[1].to_string(),
            version: request[2].to_string()
        }
    };

    println!("{:#?}", request);

    Ok(())
}


fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6969")?;

    for stream in listener.incoming(){
        handle_client(stream?)?;
    }
    Ok(())
}
