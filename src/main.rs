use std::{net::{TcpListener,TcpStream,IpAddr,Ipv4Addr,SocketAddr},io::{self,Write,Read},thread::spawn,
    path::{PathBuf, Path},fs::{read_dir,Metadata}, env};
static MONTH:[&str;12]=["Enero","Febrero","Marzo","Abril","Mayo","Junio","Julio","Agosto","Septiembre","Octubre","Noviembre","Diciembre"];
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
    NoOp,
    Pwd,
    Type,
    Pasv,
    List,
    Cwd(PathBuf),
    User(String),
    Unknown(String),
}
impl AsRef<str> for Command {
    fn as_ref(&self) -> &str {
        match *self {
            Command::Auth => "AUTH",
            Command::Syst => "SYST",
            Command::NoOp => "NOOP",
            Command::Pwd => "PWD",
            Command::Type => "TYPE",
            Command::Pasv => "PASV",
            Command::List => "LIST",
            Command::Cwd(_) => "CWD",
            Command::User(_) => "USER",
            Command::Unknown(_) => "UNKN",
        }
    }
}
impl Command {
    pub fn new(input:Vec<u8>)->io::Result<Self> {
        let mut iter=input.split(|&byte|byte==b' ');
        let mut command=iter.next().expect("comando en entrada").to_vec();
        to_uppercase(&mut command);
        let data=iter.next();
        let command=match command.as_slice() {
            b"AUTH" => Command::Auth,
            b"SYST" => Command::Syst,
            b"NOOP" => Command::NoOp,
            b"PWD" => Command::Pwd,
            b"TYPE" => Command::Type,
            b"PASV" => Command::Pasv,
            b"LIST" => Command::List,
            b"CWD" => Command::Cwd(data.map(|bytes|
                Path::new(std::str::from_utf8(bytes).unwrap()).to_path_buf()).unwrap()),
            b"USER" => Command::User(data.map(|bytes|String::from_utf8(bytes.to_vec())
                .expect("No se puede convertir bytes to string")).unwrap_or_default()),
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
    cwd:PathBuf,
    stream:TcpStream,
    name:Option<String>,
    data_writer:Option<TcpStream>,
}
impl Client {
    fn new(stream:TcpStream)->Client {
        Client { 
            cwd: PathBuf::from("/"), 
            stream, 
            name: None,
            data_writer:None,
        }
    }
    fn handle_cmd(&mut self,cmd:Command) {
        println!("====> {:?}",cmd);
        match cmd {
            Command::Auth=>send_cmd(&mut self.stream,ResultCode::CommandNotImplemented,"No implementado"),
            Command::Syst=>send_cmd(&mut self.stream,ResultCode::Ok,"No dire"),
            Command::NoOp=>send_cmd(&mut self.stream,ResultCode::Ok,"Haciendo nada"),
            Command::Pwd=>{
                let msg=format!("{}",self.cwd.to_str().unwrap_or(""));
                if !msg.is_empty() {
                    let message=format!("\"/{}\" ",msg);
                    send_cmd(&mut self.stream,ResultCode::PATHNAMECreated,&message)
                }else {
                    send_cmd(&mut self.stream,ResultCode::FileNotFound,"No existe tal archivo o directorio")
                }
            }
            Command::Type=>send_cmd(&mut self.stream,ResultCode::Ok,"Transferencia cambio bien"),
            Command::Pasv=>{
                if self.data_writer.is_some() {
                    send_cmd(&mut self.stream,ResultCode::DataConnectionAlreadyOpen,"Escuchando ya")
                }else{
                    let port=43210;
                    send_cmd(&mut self.stream,ResultCode::EnteringPassiveMode,
                        &format!("127,0,0,1,{},{}",port>>8,port&0xFF));
                    let addr=SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127,0,0,1)),port);
                    let listener=TcpListener::bind(&addr).unwrap();
                    match listener.incoming().next() {
                        Some(Ok(client))=>{self.data_writer=Some(client);}
                        _ =>{send_cmd(&mut self.stream,ResultCode::ServiceNotAvailable,"Problemas pasan");}
                    }
                }
            }
            Command::List=>{
                if let Some(ref mut data_writer) = self.data_writer {
                    let tmp=PathBuf::from(".");
                    send_cmd(&mut self.stream,ResultCode::DataConnectionOpen,"Comenzo el directorio de la lista");
                    let mut out=String::new();
                    for entry in read_dir(tmp).unwrap() {
                        if let Ok(entry) = entry {
                            add_file_info(entry.path(), &mut out);
                        }
                        send_data(data_writer, &out)
                    }
                }else{
                    send_cmd(&mut self.stream,ResultCode::ConnectionClosed,"No abrio coneccion de datos");
                }
                if self.data_writer.is_some() {
                    self.data_writer=None;
                    send_cmd(&mut self.stream,ResultCode::ClosingDataConnection,"Transferencia hecha");
                }
            }
            Command::Cwd(directory)=>self.cwd(directory),
            Command::User(username)=>{
                if username.is_empty() {
                    send_cmd(&mut self.stream,ResultCode::InvalidParameterOrArgument,"Usuario invalido")
                }else {
                    self.name=Some(username.to_owned());
                    send_cmd(&mut self.stream,ResultCode::UserLoggedIn,&format!("Bienvenido {}",username))
                }
            }
            Command::Unknown(s)=>send_cmd(&mut self.stream,ResultCode::UnknownCommand,&format!("No se implemento:'{}'",s)),
        }
    }
    fn complete_path(&self, path:PathBuf,server_root:&PathBuf)->Result<PathBuf,io::Error> {
        let directory=server_root.join(if path.has_root() {
            path.iter().skip(1).collect()
        }else {
            path
        });
        let dir=directory.canonicalize();
        if let Ok(ref dir) = dir {
            if !dir.starts_with(&server_root) {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
        }
        dir
    }
    fn cwd(&mut self,directory:PathBuf) {
        let server_root=env::current_dir().unwrap();
        let path=self.cwd.join(&directory);
        if let Ok(dir) = self.complete_path(path, &server_root) {
            if let Ok(prefix) = dir.strip_prefix(&server_root).map(|p|p.to_path_buf()) {
                self.cwd=prefix.to_path_buf();
                send_cmd(&mut self.stream, ResultCode::Ok, 
                    &format!("Directorio cambio a \"{}\"",directory.display()));
                    return
            }
        }
        send_cmd(&mut self.stream, ResultCode::FileNotFound,"No encontro archivo o carpeta");
    }
}
fn read_all_message(stream:&mut TcpStream)->Vec<u8> {
    let buf=&mut [0;1];
    let mut out=Vec::with_capacity(100);
    loop {
        match stream.read(buf) {
            Ok(received) if received > 0 => {
                if out.is_empty() && buf[0]==b' '{
                    continue;
                }
                out.push(buf[0]);
            }
            _ => return Vec::new(),
        }
        let len=out.len();
        if len > 1 && out[len-2]==b'\r' && out[len-1]==b'\n' {
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
    write!(stream,"{}",msg).unwrap()
}
fn handle_client(mut stream:TcpStream) {
    println!("nuevo cliente conectado");
    send_cmd(&mut stream, ResultCode::ServiceReadyForNewUser, "Bienvenido al servidor FTP");
    let mut client=Client::new(stream);
    loop {
        let data=read_all_message(&mut client.stream);
        if data.is_empty() {
            println!("cliente desconectado");
            break;
        }
        if let Ok(command) = Command::new(data) {
            client.handle_cmd(command);
        }else {
            println!("error con el comando del cliente");
        }
    }
}
fn send_data(stream:&mut TcpStream,s:&str) {
    write!(stream,"{}",s).unwrap();
}
fn add_file_info(path:PathBuf,out:&mut String) {
    let extra=if path.is_dir(){"/"}else{""};
    let is_dir=if path.is_dir(){"d"}else{"-"};
    let meta=match ::std::fs::metadata(&path) {
        Ok(meta)=>meta,
        _ => return,
    };
    let (time,file_size)=get_file_info(&meta);
    let path=match path.to_str() {
        Some(path)=>path,
        _ => return,
    };
    let rigths=if meta.permissions().readonly() {
        "r--r--r--"
    }else{
        "rw-rw-rw-"
    };
    let file_str=
        format!("{is_dir}{rigths} {links} {owner} {group} {size} {month} {day} {hour}:{min} {path}{extra}\r\n",
        is_dir=is_dir,
        rigths=rigths,
        links=1,
        owner="anonymous",
        group="anonymous",
        size=file_size,
        month=MONTH[time.tm_mon as usize],
        day=time.tm_mday,
        hour=time.tm_hour,
        min=time.tm_min,
        path=path,
        extra=extra);
    out.push_str(&file_str);
    println!("==> {:?}",&file_str);
}
#[macro_use]
extern crate cfg_if;
cfg_if!{
    if #[cfg(windows)]{
        fn get_file_info(meta:&Metadata)->(time::Tm,u64){
            use std::os::windows::prelude::*;
            (time::at(time::Timespec::new(meta.last_write_time())),meta.file_size())
        }
    }else{
        fn get_file_info(meta:&Metadata)->(time::Tm,u64){
            use std::os::unix::prelude::*;
            (time::at(time::Timespec::new(meta.mtime(),0)),meta.size())
        }
    }
}
fn main() {
    let listener=TcpListener::bind("127.0.0.1:1234")
        .expect("no se pudo enlazar esta direccion");
    println!("Esperando los clientes");
    for stream in listener.incoming() {
        match stream {
            Ok(stream)=>{
                spawn(move||handle_client(stream));
            }
            _ =>{println!("Un cliente trato de connectar")}
        }
    }

}