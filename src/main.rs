#[cfg(test)]
#[macro_use]
extern crate lazy_static;

#[cfg(test)]
extern crate parking_lot;

use std::env;

use async_std::io::{self, BufReader};
use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;
use async_std::task;

use serde_json::json;

macro_rules! json_log {
    ($($json:tt)+) => {
        println!("{}", json!($($json)+))
    }
}

fn get_local_addr() -> String {
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    format!("{}:{}", host, port)
}

fn parse_header(header: String) -> (String, String) {
    let values: Vec<&str> = header.splitn(2, ':').collect();
    let key = values.get(0).unwrap().to_string().to_lowercase();
    let value = values.get(1).unwrap_or(&"").to_string();
    (key, value)
}

async fn handle(stream: TcpStream) -> Result<(), io::Error> {
    stream.set_nodelay(true)?;

    let addr = stream.peer_addr()?;
    json_log!({ "peer_addr": addr });

    let (reader, writer) = &mut (&stream, &stream);

    let mut buf = BufReader::new(reader);

    // e.g. GET / HTTP/1.1\r\n
    let mut request_line = String::new();
    buf.read_line(&mut request_line).await?;
    json_log!({ "request_line": request_line });

    // reading headers
    let mut lines = buf.lines();
    let mut headers = json!({});
    while let Some(line) = lines.next().await {
        let header = line?;
        if header.is_empty() {
            break;
        }

        let (key, value) = parse_header(header);
        headers[key] = serde_json::Value::String(value);
    }
    json_log!(headers);

    // writing a response
    let res = format!("HTTP/1.1 200 OK\r\n\r\n{}\r\n", addr.ip());
    writer.write_all(res.as_bytes()).await?;
    writer.flush().await?;
    Ok(())
}

fn main() -> io::Result<()> {
    task::block_on(async {
        let addr = get_local_addr();
        let listener = TcpListener::bind(addr).await?;
        json_log!({ "local_addr": listener.local_addr()? });

        let mut incoming = listener.incoming();
        while let Some(stream) = incoming.next().await {
            match stream {
                Ok(s) => {
                    task::spawn(async {
                        handle(s).await.unwrap();
                    });
                }
                Err(e) => {
                    panic!("err: {}", e);
                }
            }
        }
        Ok(())
    })
}

#[cfg(test)]
mod test {
    use super::*;

    use std::collections::HashMap;
    use std::panic::{self, AssertUnwindSafe, UnwindSafe};

    use parking_lot::Mutex;

    // e.g. hashmap! { "key" => "value" }
    #[macro_export]
    macro_rules! hashmap(
        { $($key:expr => $value:expr),+ } => {
            {
                let mut m = HashMap::new();
                $(m.insert($key, $value);)+
                m
            }
        };
    );

    fn with<T>(
        keys: &'static str,
        vars: HashMap<&'static str, &'static str>,
        test: T,
    ) where
        T: FnOnce() + UnwindSafe,
    {
        lazy_static! {
            static ref ENV_LOCK: Mutex<()> = Mutex::new(());
        }
        let _lock = ENV_LOCK.lock();

        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            let mut origins: HashMap<&str, Result<String, env::VarError>> =
                HashMap::new();

            for key in keys.split('\n') {
                if key.is_empty() {
                    continue;
                }

                origins.insert(key, env::var(key));

                if !vars.contains_key(key) {
                    env::remove_var(key);
                } else {
                    env::set_var(key, vars.get(key).unwrap());
                }
            }

            test();

            for (key, origin) in origins {
                match origin {
                    Ok(value) => env::set_var(key, value),
                    Err(_) => env::remove_var(key),
                }
            }
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_local_addr_in_default() {
        let vars: HashMap<&'static str, &'static str> = HashMap::new();

        with(
            r#"
HOST
PORT
"#,
            vars,
            || {
                let addr = get_local_addr();
                assert_eq!(addr, "0.0.0.0:3000");
            },
        );
    }

    #[test]
    fn test_get_local_addr_is_set() {
        let vars: HashMap<&'static str, &'static str> = hashmap! {
            "HOST" => "127.0.0.1",
            "PORT" => "8000"
        };

        with(
            r#"
HOST
PORT
"#,
            vars,
            || {
                let addr = get_local_addr();
                assert_eq!(addr, "127.0.0.1:8000");
            },
        );
    }

    #[test]
    fn test_parse_header() {
        assert_eq!(
            parse_header("".to_string()),
            ("".to_string(), "".to_string())
        );

        // only key
        assert_eq!(
            parse_header("Accept".to_string()),
            ("accept".to_string(), "".to_string())
        );

        // only once
        assert_eq!(
            parse_header("Host:0.0.0.0:3000".to_string()),
            ("host".to_string(), "0.0.0.0:3000".to_string())
        );

        // does not care the case (returns always in lowercase)
        assert_eq!(
            parse_header("content-length:5".to_string()),
            ("content-length".to_string(), "5".to_string())
        );
    }
}
