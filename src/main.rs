#![feature(is_some_and)]

use std::{net::{TcpListener, TcpStream}, io::{Read, Write, BufRead, BufReader, BufWriter}, str::FromStr, fs::File, path::{Component, PathBuf}, fs};
use async_std::task::{spawn};
use strum_macros::{EnumString};
use tap::{Pipe, prelude, Tap};
use simdutf8::compat::from_utf8;
use mime_guess::mime::TEXT_PLAIN_UTF_8;

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
    body: Vec<u8>,
}

impl HttpResponse {
    // Writes the HttpResponse into a buffer, ready to be sent off to the client.
    fn to_data(&self, buf: &mut Vec<u8>) {
        *buf = [
            format!(
                "{} {} {}\r\n{}\r\n{}\r\n\r\n",
                self.version,
                self.status_code,
                self.status_text,
                self.headers.join("\r\n"),
                "Content-Length: ".to_owned() + &self.content_length.to_string(),
            ).as_bytes(),
            &*self.body, b"\r\n\r\n"
        ].concat();
    }

    // Returns self formatted as an HTTP response, excluding binary data.
    fn to_string(&self) -> String {
        return format!(
            "{} {} {}\r\n{}\r\n{}\r\n\r\n{}\r\n\r\n",
            self.version,
            self.status_code,
            self.status_text,
            self.headers.join("\r\n"),
            "Content-Length: ".to_owned() + &self.content_length.to_string(),
            from_utf8(&*self.body).unwrap_or("Binary data")
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
    }.tap_dbg(|x| eprintln!("\nREQUEST:\n{:#?}", x));

    // Read file into `body` buffer, and fetch length and MIME type
    let mut body = vec![];
    let path =
        PathBuf::from("www/".to_owned() + &match request.resource.as_str() {
            path if path.ends_with("/") => request.resource + "/" + &"index.html".to_string(),
            _ => request.resource,
        })
            // Don't allow path traversal exploits
            .components()
            .filter(|&x| x != Component::ParentDir && x != Component::RootDir)
            .collect::<PathBuf>()
            .tap_dbg(|x| eprintln!("Resolved path: {:?}", x));

    // Determine the status code to return. Content-Length is also grabbed here because it's convenient.
    let (status_code, length) = match (fs::metadata(&path), File::open(&path)) {
        (Ok(m), Err(_)) if m.is_file() => (404, 0),
        (Err(_), Err(_)) => (404, 0),
        (Ok(m), Ok(_))
            if m.is_dir()
                && fs::metadata(path.to_str().unwrap().to_owned() + "/index.html")
                .is_ok_and(|x| x.is_file())
            => (301, 0),
        (Ok(_), Ok(mut file)) => (200, file.read_to_end(&mut body)?),
        _ => (500, 0)
    };

    let mime = mime_guess::from_path(&path)
        .first()  // Assume the first MIME guess is right
        .unwrap_or(TEXT_PLAIN_UTF_8)
        .tap_dbg(|x| eprintln!("MIME at path: {}", x));

    // Turn the HttpResponse struct into a valid HTTP response, writing it into `response`
    let mut response: Vec<u8> = vec![];
    HttpResponse {
        version: "HTTP/1.1".to_string(),
        status_code,
        status_text: "Success".to_string(),
        headers: vec![format!{"Content-Type: {mime}"}],
        content_length: length as u32,
        body
    }.tap_dbg(|x| eprintln!("\nRESPONSE:\n{}", x.to_string()))
     .to_data(&mut response);

    stream_write.write_all(&mut response)?;

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
