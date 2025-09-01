use codex_redis::{Redis, resp::Resp};
use std::time::Duration;
use tempfile::tempdir;

fn resp_to_string(r: Resp) -> String {
    match r {
        Resp::Bulk(Some(b)) => String::from_utf8(b).unwrap_or_default(),
        Resp::Simple(s) => s,
        Resp::Integer(i) => i.to_string(),
        Resp::Error(e) => e,
        Resp::Bulk(None) => String::new(),
        Resp::Array(_) => String::new(),
    }
}

#[test]
fn ttl_expires_after_sleep() {
    let r = Redis::new(None);
    r.execute(&["SET".into(), "k".into(), "v".into()]);
    r.execute(&["EXPIRE".into(), "k".into(), "1".into()]);
    if let Resp::Integer(t) = r.execute(&["TTL".into(), "k".into()]) {
        assert!(t <= 1 && t >= 0);
    } else {
        panic!("unexpected resp");
    }
    std::thread::sleep(Duration::from_millis(1500));
    let ttl = r.execute(&["TTL".into(), "k".into()]);
    assert_eq!(ttl, Resp::Integer(-2));
}

#[test]
fn ttl_persists_through_aof_replay() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("aof.log");
    {
        let r = Redis::new(Some(path.clone()));
        r.execute(&["SET".into(), "k".into(), "v".into()]);
        r.execute(&["EXPIRE".into(), "k".into(), "5".into()]);
    }
    let r = Redis::new(Some(path));
    if let Resp::Integer(t) = r.execute(&["TTL".into(), "k".into()]) {
        assert!(t > 0 && t <= 5);
    } else {
        panic!("unexpected resp");
    }
}

#[test]
fn ttl_via_resp_parsing() {
    let raw =
        b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n*3\r\n$6\r\nEXPIRE\r\n$1\r\nk\r\n$1\r\n1\r\n";
    let cmds = Resp::parse_stream(raw).unwrap();
    let r = Redis::new(None);
    for cmd in cmds {
        if let Resp::Array(Some(arr)) = cmd {
            let parts: Vec<String> = arr.into_iter().map(resp_to_string).collect();
            r.execute(&parts);
        }
    }
    if let Resp::Integer(t) = r.execute(&["TTL".into(), "k".into()]) {
        assert!(t <= 1 && t >= 0);
    } else {
        panic!("unexpected resp");
    }
}
