use std::thread::{self};
use std::sync::{Arc, Mutex};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::mem;
use hyper::server::{Handler, Server, Request as HyperRequest, Response};
use hyper::header::ContentLength;
use hyper::method::{Method};
use hyper::status::StatusCode;
use serde_json;
use {Mock, SERVER_ADDRESS, Request};

#[derive(Debug)]
enum CreateMockError {
    ContentLengthMissing,
    InvalidMockResponse,
}

struct RequestHandler {
    mocks: Arc<Mutex<Vec<Mock>>>,
}

impl RequestHandler {
    pub fn new(mocks: Arc<Mutex<Vec<Mock>>>) -> Self {
        RequestHandler {
            mocks: mocks,
        }
    }

    #[allow(unused_variables)]
    fn handle_create_mock(&self, request: HyperRequest, mut response: Response) {
        match Self::mock_from(request) {
            Ok(mock) => {
                self.mocks.lock().unwrap().push(mock);
            },
            Err(e) => {
                *response.status_mut() = StatusCode::UnprocessableEntity;
                let _ = response.send(&format!("{:?}", e).as_bytes());
            }
        }
    }

    #[allow(unused_variables)]
    fn handle_delete_mocks(&self, request: HyperRequest, response: Response) {
        match request.headers.iter().find(|header| { header.name().to_lowercase() == "x-mock-id" }) {
            // Remove the element with x-mock-id
            Some(header) => {
                let id = header.value_string();
                let mut mocks = self.mocks.lock().unwrap();
                match mocks.iter().position(|mock| mock.id == id) {
                    Some(pos) => { mocks.remove(pos); },
                    None => {},
                };
            },
            // Remove all elements
            None => { self.mocks.lock().unwrap().clear(); }
        }
    }

    fn handle_default(&self, request: HyperRequest, mut response: Response) {
        let mocks = self.mocks.lock().unwrap();

        match mocks.iter().rev().find(|mock| mock.matches(&request)) {
            Some(mock) => {
                // Set the response status code
                // TODO: StatusCode::Unregistered labels everything as `<unknown status code>`
                mem::replace(response.status_mut(), StatusCode::Unregistered(mock.response.status as u16));

                // Set the response headers
                for (field, value) in &mock.response.headers {
                    response.headers_mut().set_raw(field.to_owned(), vec!(value.as_bytes().to_vec()));
                }

                // Set the response body
                response.send(mock.response.body.as_bytes()).unwrap();
            },
            None => {
                mem::replace(response.status_mut(), StatusCode::NotImplemented);
                response.send("".as_bytes()).unwrap();
            },
        };
    }

    fn mock_from(request: HyperRequest) -> Result<Mock, CreateMockError> {
        let content_length: ContentLength = *try!(request.headers.get().ok_or(CreateMockError::ContentLengthMissing));

        let mut body = String::new();
        request.take(content_length.0).read_to_string(&mut body).unwrap();

        serde_json::from_str(&body).map_err(|_| CreateMockError::InvalidMockResponse)
    }
}

impl Handler for RequestHandler {
    fn handle(&self, request: HyperRequest, response: Response) {
        match (&request.method, &*request.uri.to_string()) {
            (&Method::Post, "/mocks") => self.handle_create_mock(request, response),
            (&Method::Delete, "/mocks") => self.handle_delete_mocks(request, response),
            _ => self.handle_default(request, response),
        };
    }
}

// struct Request;
// impl ParserHandler for Request {
//    fn on_header_field(&mut self, parser: &mut Parser, header: &[u8]) -> bool {
//        println!("{}: ", str::from_utf8(header).unwrap());
//        true
//    }
//    fn on_header_value(&mut self, parser: &mut Parser, value: &[u8]) -> bool {
//        println!("\t {:?}", str::from_utf8(value).unwrap());
//        true
//    }
// }


pub fn try_start() {
    if is_listening() { return }

    start()
}

fn start() {
    // thread::spawn(move || {
    //     let mocks: Arc<Mutex<Vec<Mock>>> = Arc::new(Mutex::new(vec!()));

    //     match Server::http(SERVER_ADDRESS) {
    //         Ok(server) => { server.handle(RequestHandler::new(mocks)).unwrap(); },
    //         Err(_) => {},
    //     };
    // });

    thread::spawn(move || {
        let mut mocks: Vec<Mock> = vec!();
        let mut listener = TcpListener::bind(SERVER_ADDRESS).unwrap();
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let mut request = Request::from(&mut stream);
                    handle_request(&mut mocks, request, stream);
                },
                Err(_) => {},
            }
        }
    });

    while !is_listening() {}
}

fn is_listening() -> bool {
    TcpStream::connect(SERVER_ADDRESS).is_ok()
}

fn handle_request(mut mocks: &mut Vec<Mock>, request: Request, mut stream: TcpStream) {
    match (&*request.method, &*request.path) {
        ("POST", "/mocks") => handle_create_mock(mocks, request, stream),
        ("DELETE", "/mocks") => handle_delete_mock(mocks, request, stream),
        _ => handle_match_mock(mocks, request, stream),
    }
}

fn handle_create_mock(mut mocks: &mut Vec<Mock>, request: Request, mut stream: TcpStream) {
    println!("handle_create_mock");

    match serde_json::from_str(&request.body) {
        Ok(mock) => {
            mocks.push(mock);
            stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes());
        },
        Err(_) => {
            stream.write("HTTP/1.1 422 Unprocessable Entity\r\n\r\n".as_bytes());
        }
    }
}

fn create_mock(request: Request) -> Mock {
    serde_json::from_str(&request.body).unwrap()
}

fn handle_delete_mock(mut mocks: &mut Vec<Mock>, request: Request, mut stream: TcpStream) {
    println!("handle_delete_mock");
    stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes());
}

fn handle_match_mock(mut mocks: &mut Vec<Mock>, request: Request, mut stream: TcpStream) {
    println!("handle_match_mock");
    stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes());
}
