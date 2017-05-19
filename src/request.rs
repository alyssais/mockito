use std::io::{Read, BufRead, BufReader};
use std::mem;
use std::convert::From;
use std::default::Default;
use std::net::TcpStream;
use std::collections::HashMap;
use http_muncher::{Parser, ParserHandler};

pub struct Request {
    version: (u16, u16),
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    error: Option<String>,
    is_parsed: bool,
    last_header_field: Option<String>,
}

impl Request {
    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }

    pub fn is_parsed(&self) -> bool {
        self.is_parsed
    }

    fn content_length(&self) -> usize {
        self.headers.get("Content-Length").and_then(|length| length.parse::<usize>().ok()).unwrap_or(0)
    }
}

impl Default for Request {
    fn default() -> Self {
        Request {
            version: (1, 1),
            method: String::new(),
            path: String::new(),
            headers: HashMap::new(),
            body: String::new(),
            error: None,
            is_parsed: false,
            last_header_field: None,
        }
    }
}

impl<'a> From<&'a mut TcpStream> for Request {
    fn from(mut stream: &mut TcpStream) -> Self {
        let mut request = Request::default();
        let mut parser = Parser::request();
        let mut reader = BufReader::new(&mut stream);

        loop {
            if request.is_parsed() { break; }

            println!("read line");

            let mut line = String::new();
            let read_length = reader.read_line(&mut line).unwrap_or(0);
            if read_length == 0 { break; }

            println!("parse line");

            let parse_length = parser.parse(&mut request, line.as_bytes());
            if parse_length == 0 || parser.has_error() { break; }
        }

        if parser.has_error() {
            request.error = Some(parser.error().to_string());
            println!("{:?}", parser.error());
        } else {
            request.version = parser.http_version();
            request.method = parser.http_method().to_string();
        }

        request
    }
}

impl ParserHandler for Request {
    fn on_message_begin(&mut self, parser: &mut Parser) -> bool {
        !parser.has_error()
    }

    fn on_url(&mut self, parser: &mut Parser, value: &[u8]) -> bool {
        self.path = String::from_utf8_lossy(value).into_owned();

        !parser.has_error()
    }

    fn on_header_field(&mut self, parser: &mut Parser, value: &[u8]) -> bool {
        self.last_header_field = Some(String::from_utf8_lossy(value).into_owned());

        !parser.has_error()
    }

    fn on_header_value(&mut self, parser: &mut Parser, value: &[u8]) -> bool {
        if self.last_header_field.is_some() {
            let last_header_field = mem::replace(&mut self.last_header_field, None).unwrap();
            let last_header_value = String::from_utf8_lossy(value).into_owned();

            self.headers.insert(last_header_field, last_header_value);
        }

        !parser.has_error()
    }

    fn on_chunk_header(&mut self, _: &mut Parser) -> bool {
        println!("chunk header");

        true
    }

    fn on_headers_complete(&mut self, parser: &mut Parser) -> bool {
        println!("on_headers_complete");

        true
    }

    fn on_body(&mut self, parser: &mut Parser, value: &[u8]) -> bool {
        println!("on_body");
        println!("{:?}", value);

        self.body = String::from_utf8_lossy(value).into_owned();

        true
    }

    fn on_message_complete(&mut self, parser: &mut Parser) -> bool {
        self.is_parsed = true;

        !parser.has_error()
    }
}
