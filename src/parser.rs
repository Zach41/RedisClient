use std::io::Read;

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
            Ok(v) => Ok(v),
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
        let byte = try!(self.read_byte()) as char;
        if byte == expected {
            Ok(())
        } else {
            Err((ErrorKind::ResponseError, "Invalid byte in Response").into())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    
}
