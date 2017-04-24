use types::ToRedisArgs;

#[derive(Clone)]
enum Arg<'a> {
    Simple(Vec<u8>),
    Cursor,
    Borrowed(&'a [u8]),
}

#[derive(Clone)]
pub struct Cmd {
    args: Vec<Arg<'static>>,
    cursor: Option<u64>,
    is_ignored: bool,
}

#[derive(Clone)]
pub struct Pipeline {
    commands: Vec<Cmd>,
    transaction_mode: bool,
}

fn countdigits(mut v: usize) -> usize {
    let mut cnt = 0;
    while v > 0 {
        if v < 10 { cnt += 1; }
        else if v < 100 { cnt += 2; }
        else if v < 1000 { cnt += 3; }
        else if v < 10000 { cnt += 4; }
        v /= 10000;
        cnt += 4;
    }
    return cnt;
}

#[inline]
fn bulklen(len: usize) -> usize {
    return 1 + countdigits(len) + 2 + len + 2;
}

fn encode_commands(args: &Vec<Arg>, cursor: u64) -> Vec<u8> {
    let mut totlen = 1 + countdigits(args.len()) + 2;
    for item in args {
        totlen += bulklen(match *item {
            Arg::Cursor => countdigits(cursor as usize),
            Arg::Simple(ref v) => v.len(),
            Arg::Borrowed(ptr) => ptr.len(),
        });
    }
    let mut cmd = Vec::with_capacity(totlen);
    cmd.push('*' as u8);
    cmd.extend(args.len().to_string().as_bytes());
    cmd.push('\r' as u8);
    cmd.push('\n' as u8);

    {
        let mut encode = |item: &[u8]| {
            cmd.push('$' as u8);
            cmd.extend(item.len().to_string().as_bytes());
            cmd.push('\r' as u8);
            cmd.push('\n' as u8);
            cmd.extend(item);
            cmd.push('\r' as u8);
            cmd.push('\n' as u8);
        };
        
        for item in args {
            match *item {
                Arg::Cursor => encode(cursor.to_string().as_bytes()),
                Arg::Simple(ref v) => encode(v),
                Arg::Borrowed(ptr) => encode(ptr)
            }
        }
    }
    cmd
}

// fn encode_pipeline(cmd: &[Cmd], atomic: bool) -> Vec<u8> {
//     let mut rv = vec![];
//     if atomic {
//         rc.extend(c)
//     }
// }

impl Cmd {
    pub fn new() -> Cmd {
        Cmd {
            args: vec![],
            cursor: None,
            is_ignored: false,
        }
    }

    #[inline]
    pub fn arg<T: ToRedisArgs>(&mut self, arg: T) -> &mut Cmd {
        for item in arg.to_redis_args().into_iter() {
            self.args.push(Arg::Simple(item));
        }
        self
    }

    #[inline]
    pub fn cursor_arg(&mut self, cursor: u64) -> &mut Cmd {
        self.cursor = Some(cursor);
        self.args.push(Arg::Cursor);
        self
    }

    #[inline]
    pub fn get_packed_command(&self) -> Vec<u8> {
        encode_commands(&self.args, self.cursor.unwrap_or(0))
    }

    pub fn get_packed_command_with_cursor(&self, cursor: u64) -> Option<Vec<u8>> {
        if !self.in_scan_mode() {
            None
        } else {
            Some(encode_commands(&self.args, cursor))
        }
    }

    #[inline]
    pub fn in_scan_mode(&self) -> bool {
        self.cursor.is_some()
    }    
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_cmd_ser() {
        let mut cmd = Cmd::new();
        cmd.arg("SET").arg("my_key").arg(42);
        let serialized_cmd = "*3\r\n\
                              $3\r\nSET\r\n\
                              $6\r\nmy_key\r\n\
                              $2\r\n42\r\n".as_bytes();
        assert_eq!(cmd.get_packed_command(), serialized_cmd);
    }
}
