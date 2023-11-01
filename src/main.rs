use std::{net::{TcpListener,TcpStream,IpAddr,Ipv4Addr,SocketAddr},thread,io::{Write,Read,self},path::PathBuf};
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
#[allow(dead_code)]
enum ResultCode {
    RestartMarkerReply = 110,
    ServiceReadInXXXMinutes = 120,
    DataConnectionAlreadyOpen = 125,
    FileStatusOk = 150,
    Ok = 200,
    CommandNotImplementedSuperfluousAtThisSite = 202,
    SystemStatus = 211,
    DirectoryStatus = 212,
    FileStatus = 213,
    HelpMessage = 214,
    SystemType = 215,
    ServiceReadyForNewUser = 220,
    ServiceClosingControlConnection = 221,
    DataConnectionOpen = 225,
    ClosingDataConnection = 226,
    EnteringPassiveMode = 227,
    UserLoggedIn = 230,
    RequestedFileActionOkay = 250,
    PATHNAMECreated = 257,
    UserNameOkayNeedPassword = 331,
    NeedAccountForLogin = 332,
    RequestedFileActionPendingFurtherInformation = 350,
    ServiceNotAvailable = 421,
    CantOpenDataConnection = 425,
    ConnectionClosed = 426,
    FileBusy = 450,
    LocalErrorInProcessing = 451,
    InsufficientStorageSpace = 452,
    UnknownCommand = 500,
    InvalidParameterOrArgument = 501,
    CommandNotImplemented = 502,
    BadSequenceOfCommands = 503,
    CommandNotImplementedForThatParameter = 504,
    NotLoggedIn = 530,
    NeedAccountForStoringFiles = 532,
    FileNotFound = 550,
    PageTypeUnknown = 551,
    ExceededStorageAllocation = 552,
    FileNameNotAllowed = 553,
}
#[derive(Debug,Clone)]
enum Command {
    Auth,
    Syst,
    Pwd,
    NoOp,
    Type,
    Pasv,
    User(String),
    Unknown(String),
}
impl AsRef<str> for Command {
    fn as_ref(&self) -> &str {
        match *self {
            Command::Auth => "AUTH",
            Command::Syst => "SYST",
            Command::Pwd => "PWD",
            Command::NoOp => "NOOP",
            Command::Type => "TYPE",
            Command::Pasv => "PASV",
            Command::User(_) => "USER",
            Command::Unknown(_) => "UNKN",
        }
    }
}
impl Command {
    pub fn new(input:Vec<u8>)->io::Result<Self> {
        let mut iter=input.split(|&byte|byte == b' ');
        let mut command=iter.next().expect("comando en entrada").to_vec();
        to_uppercase(&mut command);
        let data=iter.next();
        let command=match command.as_slice() {
            b"AUTH" => Command::Auth,
            b"SYST" => Command::Syst,
            b"PWD" => Command::Pwd,
            b"NOOP" => Command::NoOp,
            b"TYPE" => Command::Type,
            b"PASV" => Command::Pasv,
            b"USER" => Command::User(data.map(|bytes|String::from_utf8(bytes.to_vec())
                .expect("no se pudo convertir bytes a string")).unwrap_or_default()),
            s => Command::Unknown(std::str::from_utf8(s).unwrap_or("").to_owned()),
        };
        Ok(command)
    }
}
fn to_uppercase(data:&mut [u8]) {
    for byte in data {
        if *byte >= 'a' as u8 && *byte <= 'z' as u8{
            *byte -= 32;
        }
    }
}
#[allow(dead_code)]
struct Client{
    cwd: PathBuf,
    stream: TcpStream,
    name: Option<String>,
    data_writer: Option<TcpStream>,
}
impl Client {
    fn new(stream:TcpStream)->Client {
        Client{
            cwd:PathBuf::from("/"),
            stream:stream,
            name:None,
            data_writer:None,
        }
    }
    fn handle_cmd(&mut self,cmd:Command) {
        println!("====> {:?}",cmd);
        match cmd {
            Command::Auth => send_cmd(&mut self.stream, ResultCode::CommandNotImplemented,
                "no implementado"),
            Command::Syst => send_cmd(&mut self.stream, ResultCode::Ok,
                "no dire"),
            Command::Pwd => {
                let msg=format!("{}",self.cwd.to_str().unwrap_or(""));
                if !msg.is_empty() {
                    let message=format!("\"/{}\" ",msg);
                    send_cmd(&mut self.stream, ResultCode::PATHNAMECreated,
                        &format!("\"/{}\" ",msg))
                }else {
                    send_cmd(&mut self.stream, ResultCode::FileNotFound, 
                        "No existe archivo o directorio")
                }
            }
            Command::NoOp => send_cmd(&mut self.stream, ResultCode::Ok,"Haciendo nada"),
            Command::Type => send_cmd(&mut self.stream, ResultCode::Ok,
                "Transferir exitosamente cambio de type"),
            Command::Pasv => {
                if  self.data_writer.is_some() {
                    send_cmd(&mut self.stream,ResultCode::DataConnectionAlreadyOpen,
                        "Ya eschuchando")
                }else {
                    let port: u16=43210;
                    send_cmd(&mut self.stream,ResultCode::EnteringPassiveMode,
                        &format!("127,0,0,1,{},{}",port>>8,port & 0xFF));
                    let addr=SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127,0,0,1)),port);
                    let listener=TcpListener::bind(&addr).unwrap();
                    match listener.incoming().next() {
                        Some(Ok(client)) =>{self.data_writer=Some(client);}
                        _ => {send_cmd(&mut self.stream,ResultCode::ServiceNotAvailable,
                            "Los problemas suceden");}
                    }
                }
            }
            Command::User(username) => {
                if username.is_empty() {
                    send_cmd(&mut self.stream, ResultCode::InvalidParameterOrArgument, 
                        "nombre de usuario invalido");
                }else {
                    self.name=Some(username.to_owned());
                    send_cmd(&mut self.stream, ResultCode::UserLoggedIn, 
                        &format!("Bienvenido {}!",username));
                }
            }
            Command::Unknown(s) => send_cmd(&mut self.stream, ResultCode::UnknownCommand, 
                &format!("No se implemento: '{}'",s)),
        }
    }
}
fn read_all_message(stream:&mut TcpStream)->Vec<u8> {
    let buf=&mut [0;1];
    let mut out=Vec::with_capacity(100);
    loop {
        match stream.read(buf) {
            Ok(received) if received > 0 =>{
                if out.is_empty() && buf[0]==b' ' {
                    continue
                }
                out.push(buf[0]);
            }
            _ => return Vec::new(),
        }
        let len=out.len();
        if len > 1 && out[len -2]==b'\r' && out[len-1]==b'\n' {
            out.pop();
            out.pop();
            return out;
        }
    }
}
fn send_cmd(stream:&mut TcpStream,code:ResultCode,message:&str) {
    let msg=if message.is_empty() {
        format!("{}\r\n",code as u32)
    }else{
        format!("{} {}\r\n",code as u32,message)
    };
    println!("<==== {}",msg);
    write!(stream,"{}",msg).unwrap();
}
fn handle_client(mut stream:TcpStream) {
    println!("Nuevo cliente conectado");
    send_cmd(&mut stream, ResultCode::ServiceReadyForNewUser, "Bienvenido al servidor FTP");
    let mut client=Client::new(stream);
    let mut loops=0;
    loop {
        loops +=1;
        println!("->{:?}",loops);
        let data=read_all_message(&mut client.stream);
        println!("data:{:?}",String::from_utf8(data.clone()));
        if data.is_empty() {
            println!("Cliente desconectado...");
            break;
        }
        if let Ok(command) = Command::new(data) {
            client.handle_cmd(command);
        }else {
            println!("Error con el comando de el cliente");
        }
        println!("<-{:?}",loops);
    }
}
fn handle(mut stream:TcpStream) {
    println!("Nuevo cliente pipe");
    send_cmd(&mut stream, ResultCode::ServiceReadyForNewUser, "Bienvenido al servidor FTP");
    let mut client=Client::new(stream);
    for _ in 0..6 {
        let data=read_all_message(&mut client.stream);
        // println!("{} {:?}",line!(),String::from_utf8(data.clone()));
        if let Ok(command) = Command::new(data) {
            client.handle_cmd(command);
        }else {
            println!("Error con el comando de el cliente");
        }        
    }
}
fn main() {
    let listener=TcpListener::bind("0.0.0.0:1234")
        .expect("fallo enlace a la direccion");
    println!("Esperando coneccion con clientes");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move||{
                    handle(stream);
                });
            }
            _ => println!("Un cliente trato de conectarse")
        }
    }
}