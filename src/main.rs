use async_std::task::spawn;
use mime_guess::mime::TEXT_PLAIN_UTF_8;
use simdutf8::compat::from_utf8;
use std::{
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Read, Write},
    net::{TcpListener, TcpStream},
    path::{Component, PathBuf},
    str::FromStr,
    env,
};
use strum_macros::EnumString;
use tap::{Tap, Pipe};

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
    UNKNOWN,
}

#[derive(Debug)]
struct HttpRequest {
    req_type: HttpRequestType,
    resource: String,
    version: String,
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

#[derive(Debug, EnumString)]
enum HttpErrorCode {
    BadRequest,
    NotFound,
    ImATeapot,
    InternalServerError,
    NotImplemented,
    HttpVersionNotSupported,
}

trait ToHttpError<T, E: std::fmt::Debug> {
    fn to_http_error(self, error_type: HttpErrorCode) -> Result<T, HttpErrorCode>;
}

impl<T, E> ToHttpError<T, E> for Result<T, E> where E: std::fmt::Debug {
    fn to_http_error(self, error_type: HttpErrorCode) -> Result<T, HttpErrorCode> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => {
                e.tap_dbg(|e| eprintln!("Creating error {:?} due to {:?}",error_type , e));
                Err(error_type)
            }
        }
    }
}

type HttpResult<T> = Result<T, HttpErrorCode>;

impl HttpErrorCode {
    fn to_http_response(&self) -> HttpResponse {
        let (status_code, status_text) = match self {
            HttpErrorCode::BadRequest => (400, "Bad Request"),
            HttpErrorCode::NotFound => (404, "Not Found"),
            HttpErrorCode::ImATeapot => (418, "Im A Teapot"),
            HttpErrorCode::InternalServerError => (500, "Internal Server Error"),
            HttpErrorCode::NotImplemented => (501, "Not Implemented"),
            HttpErrorCode::HttpVersionNotSupported => (505, "HTTP Version Not Supported"),
        };

        //TODO implement optional file resolving for errors
        // - test if a file for the given response type is located in a directory
        //   specified at runtime or through enviornment variable. If so, respond
        //   with that. Otherwise, respond with the below struct
        HttpResponse {
            version: "HTTP/1.1".into(),
            status_code,
            status_text: status_text.into(),
            headers: vec![format!{"Content-Length: {}", status_text.len() + 4}],
            //3 characters for the code itself and one for the space
            content_length: status_text.len() as u32 + 4,
            body: format!{"{status_code} {status_text}"}.into_bytes(),
        }
    }
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

async fn handle_client(request: String, cwd: String) -> HttpResult<HttpResponse> {
    // Immediately read the first line instead of waiting
    // for the EOF when the connection times out on its own.
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
            req_type: HttpRequestType::from_str(request.get(0)
                .ok_or(HttpErrorCode::BadRequest)?)
                .unwrap_or(HttpRequestType::UNKNOWN),
            resource: request.get(1)
                .ok_or( HttpErrorCode::BadRequest)?.to_string(),
            version: request.get(2)
                .ok_or( HttpErrorCode::BadRequest)?.to_string()
        }
    }.tap_dbg(|x| eprintln!("\nREQUEST:\n{:#?}", x));

    // Read file into `body` buffer, and fetch length and MIME type
    let mut body = vec![];
    let path =
        PathBuf::from(cwd.clone() + &match request.resource.as_str() {
            path if path.ends_with("/") => request.resource + "/" + &"index.html".to_string(),
            _ => request.resource,
        })
            // Don't allow path traversal exploits
            .tap_dbg(|x| eprintln!("\nResolving path:\n    Unprocessed: {:?}", x))
            .canonicalize()
            .to_http_error(HttpErrorCode::NotFound)?
            .tap_dbg(|x| eprintln!("    Canonicalized: {:?}", x))
            .pipe(|x| if x.starts_with(cwd.clone()) {x} else {PathBuf::new()})
            .tap_dbg(|x| eprintln!("    Resolved path: {:?}", x));

    //TODO make this return proper errors on all paths
    let length = File::open(&path).to_http_error(HttpErrorCode::NotFound)?
        .read_to_end(&mut body).to_http_error(HttpErrorCode::InternalServerError)?;

    let mime = mime_guess::from_path(&path)
        .first()  // Assume the first MIME guess is right
        .unwrap_or(TEXT_PLAIN_UTF_8)
        .tap_dbg(|x| eprintln!("\nMIME at path: {}", x));

    // Turn the HttpResponse struct into a valid HTTP response, writing it into `response`
    Ok(HttpResponse {
        version: "HTTP/1.1".to_string(),
        status_code: 200,
        status_text: "Success".to_string(),
        headers: vec![format!{"Content-Type: {mime}"}],
        content_length: length as u32,
        body
    }.tap_dbg(|x| eprintln!("\nRESPONSE:\n{}", x.to_string())))
     // .to_data(&mut response);

    // stream_write.write_all(&mut response).to;

}

#[async_std::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    //TODO replace std with async_std
    let listener = TcpListener::bind(String::from("0.0.0.0:") + args[1].as_str())?;
    if let Ok(cwd) = env::var("PWD") {
        env::set_current_dir(cwd).expect("no enviorment variable for path");
    }
    let cwd = PathBuf::from(env::current_dir().expect("Failed to get working directory")).into_os_string();
    let cwd = cwd.to_string_lossy().to_string();

    for stream in listener.incoming() {
        let stream = stream?;
        let cwd = cwd.clone();
        spawn(async move {
            let mut stream_read = BufReader::new(&stream);
            let mut request = String::new();
            stream_read.read_line(&mut request).unwrap();
            let result = handle_client(request, cwd).await;
            let mut stream_write = BufWriter::new(&stream);

            let mut response: Vec<u8> = vec![];
            result.unwrap_or_else(|x| x.to_http_response()).to_data(&mut response);
            stream_write.write(&response).unwrap();
            
        });
    }
    Ok(())
}
