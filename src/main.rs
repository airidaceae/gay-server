use std::{net::{TcpListener, TcpStream}, io::{Read, Write}};
use std::io::BufReader;

fn handle_client(stream: TcpStream) -> std::io::Result<()>{
    let mut buf:Vec<u8> = vec![0; 512];
    let mut tcp_data = BufReader::new(stream.try_clone().unwrap());
    tcp_data.read_to_end(&mut buf)?;
    let request = String::from_utf8(buf).unwrap();
    eprintln!("{request}");

    //fix this to make sure that [1] exists before getting it
    let path: &str = request.split(' ').collect::<Vec<&str>>()[1];
    eprintln!("{path}");
    
    Ok(())
}


fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6969")?;

    for stream in listener.incoming(){
        handle_client(stream?)?;
    }
    Ok(())
}
