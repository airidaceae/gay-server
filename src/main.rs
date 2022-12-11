use std::{
    net::{TcpListener, TcpStream},
    io::{Write, BufRead, BufReader, BufWriter},
    str::FromStr,
};
use strum_macros::{EnumString};

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

fn handle_client(stream: TcpStream) -> std::io::Result<()>{
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
    let response = HttpResponse {
        version: "HTTP/1.1".to_string(),
        status_code: 418,
        status_text: "Teapot Joke Goes Here".to_string(),
        headers: vec!["Content-Type: text/plain; charset=UTF-8".to_string()],
        content_length: 6,
        body: "hai :3".to_string(),
    };
    stream_write.write_all(response.to_string().as_bytes())?;

    Ok(())
}


fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6969")?;

    for stream in listener.incoming(){
        handle_client(stream?)?;
    }
    Ok(())
}
