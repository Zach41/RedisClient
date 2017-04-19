use std::io::{Read, Cursor};

use types::{Value, ErrorKind, RedisResult};

pub struct Parser<T> {
    reader: T,
}

impl<T: Read> Parser<T> {
    pub fn new(reader: T) -> Parser<T> {
        Parser {
            reader: reader
        }
    }

    pub fn parse_value(&mut self) -> RedisResult<Value> {
        let byte = try!(self.read_byte());
        match byte as char {
            '+' => self.parse_status_value(),
            '-' => self.parse_error(),
            ':' => self.parse_int_value(),
            '$' => self.parse_data_value(),
            '*' => self.parse_bulk_value(),
            _ => Err((ErrorKind::ResponseError, "Invalid response when parsing value").into())
        }
    }

    fn parse_int_value(&mut self) -> RedisResult<Value> {
        Ok(Value::Int(try!(self.read_int_value())))
    }

    fn parse_status_value(&mut self) -> RedisResult<Value> {
        let line = try!(self.read_string_line());
        if line == "OK" {
            Ok(Value::Okay)
        } else {
            Ok(Value::Status(line))
        }
    }

    fn parse_data_value(&mut self) -> RedisResult<Value> {
        let length = try!(self.read_int_value());
        if length < 0 {
            Ok(Value::Nil)
        } else {
            let rv = try!(self.read(length as usize));
            try!(self.expect_char('\r'));
            try!(self.expect_char('\n'));
            Ok(Value::Data(rv))
        }
    }

    fn parse_bulk_value(&mut self) -> RedisResult<Value> {
        let length = try!(self.read_int_value());
        if length < 0 {
            Ok(Value::Nil)
        } else {
            let mut rv = vec![];
            rv.reserve(length as usize);
            for _ in 0..length {
                rv.push(try!(self.parse_value()));
            }
            Ok(Value::Bulk(rv))
        }
    }

    fn parse_error(&mut self) -> RedisResult<Value> {
        let desc = "An error was signaled by the server";
        let line = try!(self.read_string_line());
        let mut pieces = line.splitn(2, ' ');
        let kind = match pieces.next().unwrap() {
            "ERR" => ErrorKind::ResponseError,
            "EXECABORT" => ErrorKind::ExecAbortError,
            "LOADING" => ErrorKind::BusyLoadingError,
            "NOSCRIPT" => ErrorKind::NoScriptError,
            code => ErrorKind::ExtensionError(code.to_string()),
        };
        match pieces.next() {
            Some(detail) => Err((kind, desc, detail.to_string()).into()),
            None => Err((kind, desc).into())
        }
    }

    // helpers
    fn read_byte(&mut self) -> RedisResult<u8> {
        let mut byte: [u8; 1] = [0u8];
        let nread = try!(self.reader.read(&mut byte));
        if nread < 1 {
            Err((ErrorKind::ResponseError, "Could not read enough bytes").into())
        } else {
            Ok(byte[0])
        }        
    }

    fn read_string_line(&mut self) -> RedisResult<String> {
        match String::from_utf8(try!(self.read_line())) {
            Ok(s) => Ok(s),
            Err(e) => Err(e.into())
        } 
    }

    fn read_int_value(&mut self) -> RedisResult<i64> {        
        let line = try!(self.read_string_line());
        match line.trim().parse::<i64>() {
            Ok(v) => {
                trace!("Read int value: {}", v);
                Ok(v)
            }
            Err(_) => Err((ErrorKind::ResponseError, "Expected integer, got garbage").into())
        }
    }

    fn read_line(&mut self) -> RedisResult<Vec<u8>> {
        let mut rv = vec![];

        loop {
            let b = try!(self.read_byte());
            match b as char {
                '\r' => {
                    try!(self.expect_char('\n'));
                    break;
                },
                '\n' => {
                    break;
                },
                _ => rv.push(b)
            }
        }
        Ok(rv)
    }

    fn read(&mut self, size: usize) -> RedisResult<Vec<u8>> {
        trace!("Read {} bytes", size);
        let mut rv = vec![0; size];
        let mut i = 0;
        while i < size {
            let nread = {
                let buf = &mut rv[i..];
                self.reader.read(buf)
            };
            match nread {
                Ok(n) if n > 0 => {
                    i += n;
                },
                Ok(_) => {
                    return Err((ErrorKind::ResponseError, "Could not read enought bytes").into());
                },
                Err(e) => {
                    return Err(e.into())
                }
            }
        }
        Ok(rv)
    }

    fn expect_char(&mut self, expected: char) -> RedisResult<()> {
        trace!("Expecting Char: {}", expected);
        let byte = try!(self.read_byte()) as char;
        if byte == expected {
            Ok(())
        } else {
            Err((ErrorKind::ResponseError, "Invalid byte in Response").into())
        }
    }
}

pub fn parse_redis_value(bytes: &[u8]) -> RedisResult<Value> {
    let mut parser = Parser::new(Cursor::new(bytes));
    parser.parse_value()
}

#[cfg(test)]
mod test {
    #[cfg(test)]
    extern crate env_logger;
    
    use super::*;
    use types::RedisError;

    #[test]
    fn test_parse_int() {
        let bytes = ":12\r\n".as_bytes();
        let value = parse_redis_value(bytes).unwrap();
        assert_eq!(value, Value::Int(12i64));
    }

    #[test]
    fn test_parse_ok() {
        let bytes = "+OK\r\n".as_bytes();
        assert_eq!(Value::Okay, parse_redis_value(bytes).unwrap());
    }

    #[test]
    fn test_parse_nil() {
        let bytes1 = "$-1\r\n".as_bytes();
        let bytes2 = "*-1\r\n".as_bytes();
        assert_eq!(Value::Nil, parse_redis_value(bytes1).unwrap());
        assert_eq!(Value::Nil, parse_redis_value(bytes2).unwrap());
    }

    #[test]
    fn test_parse_string() {
        // env_logger::init().unwrap();
        let bytes_nil = "$0\r\n\r\n".as_bytes();
        let bytes = "$12hello, redis\r\n".as_bytes();
        assert_eq!(Value::Data(vec![]), parse_redis_value(bytes_nil).unwrap());
        // assert_eq!(Value::Data("hello, redis".as_bytes().to_vec()),
        //            parse_redis_value(bytes).unwrap());
    }

    #[test]
    fn test_parse_error() {
        env_logger::init().unwrap();
        let msg1 = "unknown command 'foobar'";
        let msg2 = "Operation against a key holding the wrong kind of value";
        let bytes1 = "-ERR unknown command 'foobar'\r\n".as_bytes();
        let bytes2 = "-WRONGTYPE Operation against a key holding the wrong kind of value\r\n".as_bytes();

        let err1 = parse_redis_value(bytes1).err().unwrap();       
        assert_eq!(RedisError::from((ErrorKind::ResponseError,
                    "An error was signaled by the server",
                    msg1.to_string())),
                   err1);

        let err2 = parse_redis_value(bytes2).err().unwrap();               
        assert_eq!(RedisError::from((ErrorKind::ExtensionError("WRONGTYPE".to_string()),
                    "An error was signaled by the server",
                    msg2.to_string())),
                   err2);
    }

    #[test]
    fn test_bulk() {
        let bulk0 = "*0\r\n".as_bytes();
        let bulk1 = "*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n".as_bytes();
        let bulk2 = "*5\r\n:1\r\n:2\r\n:3\r\n:4\r\n$6\r\nfoobar\r\n".as_bytes();
        let bulk3 = "*-1\r\n".as_bytes();

        assert_eq!(Value::Bulk(vec![]),
                   parse_redis_value(bulk0).unwrap());
        assert_eq!(Value::Bulk(vec![Value::Data("foo".as_bytes().to_vec()),
                                    Value::Data("bar".as_bytes().to_vec())]),
                   parse_redis_value(bulk1).unwrap());
        assert_eq!(Value::Bulk(vec![Value::Int(1i64), Value::Int(2i64), Value::Int(3i64), Value::Int(4i64),
                                Value::Data("foobar".as_bytes().to_vec())]),
                   parse_redis_value(bulk2).unwrap());
        assert_eq!(Value::Nil, parse_redis_value(bulk3).unwrap());
    }
}
