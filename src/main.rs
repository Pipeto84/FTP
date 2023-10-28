use std::net::TcpListener;
use std::io::Write;

fn main() {
    let listener=TcpListener::bind("0.0.0.0:1234")
        .expect("fallo enlace a la direccion");
    println!("Esperando coneccion con clientes");
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("Nuevo cliente");
                if let Err(_) = stream.write(b"pipe") {
                    println!("Fallo enviar pipe");
                }
            }
            _ => println!("Un cliente trato de conectarse")
        }
    }
}