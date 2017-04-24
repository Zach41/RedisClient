use std::io;
use std::error::Error;
use std::string::FromUtf8Error;

/// Error kinds
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum ErrorKind {
    /// invalid server response
    ResponseError,
    /// The authentication with server failed
    AuthenticationFailed,
    /// Operation failed because of a type mismatch
    TypeError,
    /// A script execution was aborted
    ExecAbortError,
    /// The server can't response because it's busy
    BusyLoadingError,
    /// A script that was requested does not actually exists
    NoScriptError,
    /// A error that is unknown to the library.
    ExtensionError(String),
    /// IoError
    IoError
}

/// Redis Value Enum
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Value {
    /// nil response
    Nil,
    /// integer response
    Int(i64),
    /// an arbiary binary data
    Data(Vec<u8>),
    /// nested structures response
    Bulk(Vec<Value>),
    /// a status response, normally a string
    Status(String),
    /// "OK" response
    Okay,    
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct RedisError {
    pub kind: ErrorKind,
    pub desc: &'static str,
    pub detail: Option<String>,
}

impl From<io::Error> for RedisError {
    fn from(err: io::Error) -> RedisError {
        RedisError {
            kind: ErrorKind::IoError,
            desc: "An internal IO error occurred",
            detail: Some(err.description().to_string())
        }
    }
}

impl From<FromUtf8Error> for RedisError {
    fn from(err: FromUtf8Error) -> RedisError {
        RedisError {
            kind: ErrorKind::TypeError,
            desc: "Invalid UTF-8",
            detail: Some(err.description().to_string())
        }
    }
}

impl From<(ErrorKind, &'static str)> for RedisError {
    fn from((kind, desc): (ErrorKind, &'static str)) -> RedisError {
        RedisError {
            kind: kind,
            desc: desc,
            detail: None,
        }
    }
}

impl From<(ErrorKind, &'static str, String)> for RedisError {
    fn from((kind, desc, detail): (ErrorKind, &'static str, String)) -> RedisError {
        RedisError {
            kind: kind,
            desc: desc,
            detail: Some(detail),
        }
    }
}

pub type RedisResult<T> = Result<T, RedisError>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NumericBehavior {
    NonNumeric,
    NumericInteger,
    NumericFloat,
}
/// This trait used to convert a value into one or multiple redis commands arguments
pub trait ToRedisArgs: Sized {
    fn to_redis_args(&self) -> Vec<Vec<u8>>;

    fn describe_numberic_behavior(&self) -> NumericBehavior {
        NumericBehavior::NonNumeric
    }

    fn is_single_arg(&self) -> bool {
        true
    }

    fn make_arg_vec(items: &[Self]) -> Vec<Vec<u8>> {
        let mut rv = vec![];
        for item in items.iter() {
            rv.extend(item.to_redis_args().into_iter());
        }
        rv
    }

    fn is_single_vec_arg(items: &[Self]) -> bool {
        items.len() == 1 && items[0].is_single_arg()
    }
}

macro_rules! string_based_to_redis_args {
    ($t:ty, $numeric:expr) => {
        impl ToRedisArgs for $t {
            fn to_redis_args(&self) -> Vec<Vec<u8>> {
                let s = self.to_string();
                vec![s.as_bytes().to_vec()]
            }

            fn describe_numberic_behavior(&self) -> NumericBehavior {
                $numeric
            }
        }
    }
}

impl ToRedisArgs for u8 {
    fn to_redis_args(&self) -> Vec<Vec<u8>> {
        let s = self.to_string();
        vec![s.as_bytes().to_vec()]
    }

    fn make_arg_vec(items: &[u8]) -> Vec<Vec<u8>> {
        vec![items.to_vec()]
    }

    fn is_single_vec_arg(_items: &[u8]) -> bool {
        true
    }
}

string_based_to_redis_args!(i8, NumericBehavior::NumericInteger);
string_based_to_redis_args!(i16, NumericBehavior::NumericInteger);
string_based_to_redis_args!(u16, NumericBehavior::NumericInteger);
string_based_to_redis_args!(i32, NumericBehavior::NumericInteger);
string_based_to_redis_args!(u32, NumericBehavior::NumericInteger);
string_based_to_redis_args!(i64, NumericBehavior::NumericInteger);
string_based_to_redis_args!(u64, NumericBehavior::NumericInteger);
string_based_to_redis_args!(f32, NumericBehavior::NumericFloat);
string_based_to_redis_args!(f64, NumericBehavior::NumericFloat);
string_based_to_redis_args!(isize, NumericBehavior::NumericInteger);
string_based_to_redis_args!(usize, NumericBehavior::NumericInteger);
string_based_to_redis_args!(bool, NumericBehavior::NonNumeric);

impl ToRedisArgs for String {
    fn to_redis_args(&self) -> Vec<Vec<u8>> {
        vec![self.as_bytes().to_vec()]
    }
}

impl<'a> ToRedisArgs for &'a String {
    fn to_redis_args(&self) -> Vec<Vec<u8>> {
        vec![self.as_bytes().to_vec()]
    }
}

impl<'a> ToRedisArgs for &'a str {
    fn to_redis_args(&self) -> Vec<Vec<u8>> {
        vec![self.as_bytes().to_vec()]
    }
}

impl<T: ToRedisArgs> ToRedisArgs for Vec<T> {
    fn to_redis_args(&self) -> Vec<Vec<u8>> {
        ToRedisArgs::make_arg_vec(self)
    }

    fn is_single_arg(&self) -> bool {
        ToRedisArgs::is_single_vec_arg(&self[..])
    }
}

impl<'a, T: ToRedisArgs> ToRedisArgs for &'a [T] {
    fn to_redis_args(&self) -> Vec<Vec<u8>> {
        ToRedisArgs::make_arg_vec(*self)
    }

    fn is_single_arg(&self) -> bool {
        ToRedisArgs::is_single_vec_arg(*self)
    }
}

impl<T: ToRedisArgs> ToRedisArgs for Option<T> {
    fn to_redis_args(&self) -> Vec<Vec<u8>> {
        match *self {
            Some(ref v) => v.to_redis_args(),
            None => vec![]
        }
    }

    fn describe_numberic_behavior(&self) -> NumericBehavior {
        match *self {
            Some(ref v) => v.describe_numberic_behavior(),
            None => NumericBehavior::NonNumeric,
        }
    }

    fn is_single_arg(&self) -> bool {
        match *self {
            Some(ref v) => v.is_single_arg(),
            None => false
        }
    }
}

macro_rules! to_redis_args_for_tuple {
    () => ();
    ($($name: ident,)+) => {
        impl<$($name: ToRedisArgs), *> ToRedisArgs for ($($name, )*) {
            #[allow(non_snake_case)]
            fn to_redis_args(&self) -> Vec<Vec<u8>> {
                let ($(ref $name, )*) = *self;
                let mut rv = vec![];
                $(rv.extend($name.to_redis_args().into_iter());)*
                rv
            }

            #[allow(non_snake_case, unused_variables)]
            fn is_single_arg(&self) -> bool {
                let mut n = 0;
                $(let $name = (); n += 1;)*
                n == 1
            }
        }
        to_redis_args_for_tuple_peel!($($name, )*);
    }
}

macro_rules! to_redis_args_for_tuple_peel {
    ($name: ident, $($other: ident, )*) => {
        to_redis_args_for_tuple!($($other,)*);
    }
}

to_redis_args_for_tuple! (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, );

macro_rules! to_redis_args_for_array {
    ($($N:expr)+) => {
        $(
            impl<'a, T: ToRedisArgs> ToRedisArgs for &'a [T; $N] {
                fn to_redis_args(&self) -> Vec<Vec<u8>> {
                    ToRedisArgs::make_arg_vec(*self)
                }

                fn is_single_arg(&self) -> bool {
                    ToRedisArgs::is_single_vec_arg(*self)
                }
            }
        )+
    }
}

to_redis_args_for_array! (
    0   1    2    3    4    5    6    7    8     9
   10  11   12   13   14   15   16   17   18    19
   20  21   22   23   24   25   26   27   28    29
   30  31   32
);

