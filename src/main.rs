use std::{
    net::{TcpListener, TcpStream},
    io::{Read, Write, BufRead, BufReader}
};
use std::io::BufWriter;
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
    let mut stream_read = BufReader::new(&stream);

    // Immediately read first line instead of waiting for the connection to
    // close and send EOF
    let mut request = String::new();
    stream_read.read_line(&mut request);
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

    let mut stream_write = BufWriter::new(&stream);

    stream_write.write_all("HTTP/1.1 418 Teapot Joke Goes Here\r\nContent-Type: text/plain; charset=UTF-8\r\nContent-Length: 6\r\n\r\nhai :3\r\n\r\n".as_bytes());
    Ok(())
}


fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6969")?;

    for stream in listener.incoming(){
        handle_client(stream?)?;
    }
    Ok(())
}
