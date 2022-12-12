use std::{
    net::{TcpListener, TcpStream},
    io::{Write, BufRead, BufReader, BufWriter},
    str::FromStr,
};
use std::fs::File;
use std::io::Read;
use strum_macros::{EnumString};
use async_std::task::{spawn};

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

#[derive(Debug)]
struct HttpResponse {
    version: String,
    status_code: u32,
    status_text: String,
    headers: Vec<String>,
    content_length: u32,
    body: String,
}

impl HttpResponse {
    fn to_string(&self) -> String {
        return format!(
            "{} {} {}\r\n{}{}\r\n\r\n{}\r\n\r\n",
            self.version,
            self.status_code,
            self.status_text,
            self.headers.join("\r\n"),
            self.content_length,
            self.body
        )
    }
}

async fn handle_client(stream: TcpStream) -> std::io::Result<()>{
    let mut stream_read = BufReader::new(&stream);
    let mut stream_write = BufWriter::new(&stream);

    // Immediately read the first line instead of waiting
    // for the EOF when the connection times out on its own.
    let mut request = String::new();
    stream_read.read_line(&mut request)?;
    let request = request.to_string();

    let request: HttpRequest = {
        // Create a vec for the 3 fields in the first line of the HTTP request header
        let request = request
            .split(' ')
            .take(3)  // Immediately defeats every malformed request attack
            .map(|x| x.trim())  // Remove extraneous line breaks and whatnot
            .collect::<Vec<&str>>();

        // Shove our request vec into a struct
        HttpRequest {
            req_type: HttpRequestType::from_str(request[0]).unwrap_or(HttpRequestType::UNKNOWN),
            resource: request[1].to_string(),
            version: request[2].to_string()
        }
    };

    println!("{:#?}", request);

    // Return a basic response. Nothing crazy for now, just making sure it all works.
    let mut page = String::new();
    let length = File::open("assets/index.html")?.read_to_string(&mut page)?;
    println!("{}", page);
    let response = HttpResponse {
        version: "HTTP/1.1".to_string(),
        status_code: 200,
        status_text: "Success".to_string(),
        headers: vec!["Content-Type: text/html; charset=UTF-8".to_string()],
        content_length: length as u32,
        body: page
    };
    stream_write.write_all(response.to_string().as_bytes())?;

    Ok(())
}

#[async_std::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6969")?;

    for stream in listener.incoming() {
        spawn(handle_client(stream?));
    }
    Ok(())
}
