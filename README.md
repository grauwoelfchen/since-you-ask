# Since You Ask

A server returns your current IP address ;)

> What is my IP address?
> Well, since you ask, this is yours.

## Build

```zsh
% make build
```

## Usage

```zsh
: run the built server
% ./target/debug/since-you-ask
{"local_addr":"0.0.0.0:3000"}

: access it
% curl http://0.0.0.0:3000
127.0.0.1
```

Then, you will get outputs like the followings:

```zsh
% ./target/debug/since-you-ask
...
{"peer_addr":"127.0.0.1:38082"}
{"request_line":"GET / HTTP/1.1\r\n"}
{"accept":" */*","host":" 0.0.0.0:3000","user-agent":" curl/7.81.0"}
```

## License

`MIT OR Apache-2.0`

```txt
Since You Ask
Copyright (c) 2022 Yasuhiro Яша Asaka
```
