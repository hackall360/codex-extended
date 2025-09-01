pub mod resp;

use resp::Resp;
use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions, read};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct Redis {
    inner: Arc<Inner>,
}

struct Inner {
    store: Mutex<HashMap<String, Value>>,
    expires: Mutex<HashMap<String, Instant>>,
    pubsub: Mutex<HashMap<String, broadcast::Sender<String>>>,
    aof: Mutex<Option<File>>,
    aof_path: Option<PathBuf>,
    vectors: Mutex<HashMap<String, codex_redis_vector::Index>>,
}

#[derive(Clone, Debug)]
enum Value {
    String(String),
    Hash(HashMap<String, String>),
    Set(HashSet<String>),
}

impl Redis {
    pub fn new(aof_path: Option<PathBuf>) -> Self {
        let inner = Arc::new(Inner {
            store: Mutex::new(HashMap::new()),
            expires: Mutex::new(HashMap::new()),
            pubsub: Mutex::new(HashMap::new()),
            aof: Mutex::new(None),
            aof_path: aof_path.clone(),
            vectors: Mutex::new(HashMap::new()),
        });
        let redis = Redis {
            inner: inner.clone(),
        };
        if let Some(path) = aof_path.clone() {
            // attempt to load vector index snapshots first
            if let Some(dir) = path.parent() {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                            if name.starts_with("vec_") && name.ends_with(".hnsw") {
                                let key = &name[4..name.len() - 5];
                                if let Ok(bytes) = read(&p) {
                                    if let Ok(idx) = codex_redis_vector::Index::load(&bytes) {
                                        inner.vectors.lock().unwrap().insert(key.to_string(), idx);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .read(true)
                .open(&path)
                .unwrap();
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).unwrap();
            if let Ok(cmds) = Resp::parse_stream(&buf) {
                for cmd in cmds {
                    if let Resp::Array(Some(arr)) = cmd {
                        let parts: Vec<String> = arr.into_iter().map(resp_to_string).collect();
                        redis.execute_inner(&parts, false);
                    }
                }
            }
            *redis.inner.aof.lock().unwrap() = Some(file);
        }
        redis.spawn_sweeper();
        redis
    }

    fn spawn_sweeper(&self) {
        let inner = self.inner.clone();
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(1));
                let now = Instant::now();
                let mut exp = inner.expires.lock().unwrap();
                let mut store = inner.store.lock().unwrap();
                let keys: Vec<String> = exp
                    .iter()
                    .filter(|(_, t)| **t <= now)
                    .map(|(k, _)| k.clone())
                    .collect();
                for k in keys {
                    exp.remove(&k);
                    store.remove(&k);
                }
            }
        });
    }

    pub fn execute(&self, cmd: &[String]) -> Resp {
        self.execute_inner(cmd, true)
    }

    fn execute_inner(&self, cmd: &[String], log: bool) -> Resp {
        if cmd.is_empty() {
            return Resp::Error("ERR empty command".into());
        }
        let c = cmd[0].to_uppercase();
        match c.as_str() {
            "SET" => {
                if cmd.len() < 3 {
                    return Resp::Error("ERR wrong number of arguments".into());
                }
                let key = cmd[1].clone();
                let val = cmd[2].clone();
                self.inner
                    .store
                    .lock()
                    .unwrap()
                    .insert(key.clone(), Value::String(val));
                self.inner.expires.lock().unwrap().remove(&key);
                if log {
                    self.log(cmd);
                }
                Resp::Simple("OK".into())
            }
            "GET" => {
                if cmd.len() < 2 {
                    return Resp::Error("ERR wrong number of arguments".into());
                }
                let key = &cmd[1];
                self.cleanup_key(key);
                if let Some(Value::String(v)) = self.inner.store.lock().unwrap().get(key).cloned() {
                    Resp::Bulk(Some(v.into_bytes()))
                } else {
                    Resp::Bulk(None)
                }
            }
            "DEL" => {
                let mut count = 0;
                for k in &cmd[1..] {
                    self.cleanup_key(k);
                    if self.inner.store.lock().unwrap().remove(k).is_some() {
                        self.inner.expires.lock().unwrap().remove(k);
                        count += 1;
                    }
                }
                if log {
                    self.log(cmd);
                }
                Resp::Integer(count)
            }
            "EXISTS" => {
                let mut count = 0;
                for k in &cmd[1..] {
                    self.cleanup_key(k);
                    if self.inner.store.lock().unwrap().contains_key(k) {
                        count += 1;
                    }
                }
                Resp::Integer(count)
            }
            "HSET" => {
                if cmd.len() < 4 {
                    return Resp::Error("ERR wrong number of arguments".into());
                }
                let key = &cmd[1];
                let field = &cmd[2];
                let value = &cmd[3];
                self.cleanup_key(key);
                let mut store = self.inner.store.lock().unwrap();
                let map = store
                    .entry(key.clone())
                    .or_insert_with(|| Value::Hash(HashMap::new()));
                let inserted = if let Value::Hash(h) = map {
                    h.insert(field.clone(), value.clone()).is_none()
                } else {
                    *map = Value::Hash(HashMap::new());
                    if let Value::Hash(h) = map {
                        h.insert(field.clone(), value.clone());
                    }
                    true
                };
                drop(store);
                if log {
                    self.log(cmd);
                }
                Resp::Integer(if inserted { 1 } else { 0 })
            }
            "HGET" => {
                if cmd.len() < 3 {
                    return Resp::Error("ERR wrong number of arguments".into());
                }
                let key = &cmd[1];
                let field = &cmd[2];
                self.cleanup_key(key);
                let store = self.inner.store.lock().unwrap();
                if let Some(Value::Hash(h)) = store.get(key) {
                    if let Some(v) = h.get(field) {
                        return Resp::Bulk(Some(v.clone().into_bytes()));
                    }
                }
                Resp::Bulk(None)
            }
            "HDEL" => {
                if cmd.len() < 3 {
                    return Resp::Error("ERR wrong number of arguments".into());
                }
                let key = &cmd[1];
                let field = &cmd[2];
                self.cleanup_key(key);
                let mut removed = 0;
                if let Some(Value::Hash(h)) = self.inner.store.lock().unwrap().get_mut(key) {
                    if h.remove(field).is_some() {
                        removed = 1;
                    }
                }
                if log {
                    self.log(cmd);
                }
                Resp::Integer(removed)
            }
            "HGETALL" => {
                if cmd.len() < 2 {
                    return Resp::Error("ERR wrong number of arguments".into());
                }
                let key = &cmd[1];
                self.cleanup_key(key);
                let store = self.inner.store.lock().unwrap();
                if let Some(Value::Hash(h)) = store.get(key) {
                    let mut arr = Vec::new();
                    for (k, v) in h {
                        arr.push(Resp::Bulk(Some(k.clone().into_bytes())));
                        arr.push(Resp::Bulk(Some(v.clone().into_bytes())));
                    }
                    Resp::Array(Some(arr))
                } else {
                    Resp::Array(Some(vec![]))
                }
            }
            "SADD" => {
                if cmd.len() < 3 {
                    return Resp::Error("ERR wrong number of arguments".into());
                }
                let key = &cmd[1];
                self.cleanup_key(key);
                let mut store = self.inner.store.lock().unwrap();
                let set = store
                    .entry(key.clone())
                    .or_insert_with(|| Value::Set(HashSet::new()));
                let mut added = 0;
                if let Value::Set(s) = set {
                    for m in &cmd[2..] {
                        if s.insert(m.clone()) {
                            added += 1;
                        }
                    }
                }
                drop(store);
                if log {
                    self.log(cmd);
                }
                Resp::Integer(added)
            }
            "SREM" => {
                if cmd.len() < 3 {
                    return Resp::Error("ERR wrong number of arguments".into());
                }
                let key = &cmd[1];
                self.cleanup_key(key);
                let mut removed = 0;
                if let Some(Value::Set(s)) = self.inner.store.lock().unwrap().get_mut(key) {
                    for m in &cmd[2..] {
                        if s.remove(m) {
                            removed += 1;
                        }
                    }
                }
                if log {
                    self.log(cmd);
                }
                Resp::Integer(removed)
            }
            "SMEMBERS" => {
                if cmd.len() < 2 {
                    return Resp::Error("ERR wrong number of arguments".into());
                }
                let key = &cmd[1];
                self.cleanup_key(key);
                let store = self.inner.store.lock().unwrap();
                if let Some(Value::Set(s)) = store.get(key) {
                    let arr = s
                        .iter()
                        .cloned()
                        .map(|m| Resp::Bulk(Some(m.into_bytes())))
                        .collect();
                    Resp::Array(Some(arr))
                } else {
                    Resp::Array(Some(vec![]))
                }
            }
            "EXPIRE" => {
                if cmd.len() < 3 {
                    return Resp::Error("ERR wrong number of arguments".into());
                }
                let key = &cmd[1];
                let sec: i64 = cmd[2].parse().unwrap_or(0);
                self.cleanup_key(key);
                if self.inner.store.lock().unwrap().contains_key(key) {
                    self.inner.expires.lock().unwrap().insert(
                        key.clone(),
                        Instant::now() + Duration::from_secs(sec as u64),
                    );
                    if log {
                        self.log(cmd);
                    }
                    Resp::Integer(1)
                } else {
                    Resp::Integer(0)
                }
            }
            "PEXPIRE" => {
                if cmd.len() < 3 {
                    return Resp::Error("ERR wrong number of arguments".into());
                }
                let key = &cmd[1];
                let ms: i64 = cmd[2].parse().unwrap_or(0);
                self.cleanup_key(key);
                if self.inner.store.lock().unwrap().contains_key(key) {
                    self.inner.expires.lock().unwrap().insert(
                        key.clone(),
                        Instant::now() + Duration::from_millis(ms as u64),
                    );
                    if log {
                        self.log(cmd);
                    }
                    Resp::Integer(1)
                } else {
                    Resp::Integer(0)
                }
            }
            "TTL" => {
                if cmd.len() < 2 {
                    return Resp::Error("ERR wrong number of arguments".into());
                }
                let key = &cmd[1];
                self.cleanup_key(key);
                if !self.inner.store.lock().unwrap().contains_key(key) {
                    return Resp::Integer(-2);
                }
                if let Some(t) = self.inner.expires.lock().unwrap().get(key) {
                    Resp::Integer((*t - Instant::now()).as_secs() as i64)
                } else {
                    Resp::Integer(-1)
                }
            }
            "PERSIST" => {
                if cmd.len() < 2 {
                    return Resp::Error("ERR wrong number of arguments".into());
                }
                let key = &cmd[1];
                self.cleanup_key(key);
                let removed = self.inner.expires.lock().unwrap().remove(key).is_some();
                if removed && log {
                    self.log(cmd);
                }
                Resp::Integer(if removed { 1 } else { 0 })
            }
            "PUBLISH" => {
                if cmd.len() < 3 {
                    return Resp::Error("ERR wrong number of arguments".into());
                }
                let ch = &cmd[1];
                let msg = cmd[2].clone();
                let n = self.publish(ch, msg) as i64;
                Resp::Integer(n)
            }
            "VEC.CREATE" => {
                if cmd.len() < 5 {
                    return Resp::Error("ERR wrong number of arguments".into());
                }
                let key = cmd[1].clone();
                let dim: usize = cmd[2].parse().unwrap_or(0);
                let m: usize = cmd[3].parse().unwrap_or(0);
                let efc: usize = cmd[4].parse().unwrap_or(0);
                if !log && self.inner.vectors.lock().unwrap().contains_key(&key) {
                    return Resp::Simple("OK".into());
                }
                match codex_redis_vector::Index::new(dim, m, efc) {
                    Ok(idx) => {
                        self.inner.vectors.lock().unwrap().insert(key.clone(), idx);
                        if log {
                            self.log(cmd);
                        }
                        Resp::Simple("OK".into())
                    }
                    Err(e) => Resp::Error(format!("ERR {e}")),
                }
            }
            "VEC.ADD" => {
                if cmd.len() < 5 {
                    return Resp::Error("ERR wrong number of arguments".into());
                }
                let key = cmd[1].clone();
                let id: u32 = cmd[2].parse().unwrap_or(0);
                let vec: Vec<f32> = cmd[3].split(',').filter_map(|s| s.parse().ok()).collect();
                let payload = cmd[4].clone();
                let mut vecs = self.inner.vectors.lock().unwrap();
                if let Some(index) = vecs.get_mut(&key) {
                    match index.add(id, vec) {
                        Ok(_) => {
                            self.inner
                                .store
                                .lock()
                                .unwrap()
                                .insert(format!("doc:{id}"), Value::String(payload.clone()));
                            if log {
                                self.log(cmd);
                            }
                            Resp::Simple("OK".into())
                        }
                        Err(e) => Resp::Error(format!("ERR {e}")),
                    }
                } else {
                    Resp::Error("ERR no such index".into())
                }
            }
            "VEC.SEARCH" => {
                if cmd.len() < 5 {
                    return Resp::Error("ERR wrong number of arguments".into());
                }
                let key = cmd[1].clone();
                let query: Vec<f32> = cmd[2].split(',').filter_map(|s| s.parse().ok()).collect();
                let k: usize = cmd[3].parse().unwrap_or(0);
                let ef: usize = cmd[4].parse().unwrap_or(k);
                let vecs = self.inner.vectors.lock().unwrap();
                if let Some(index) = vecs.get(&key) {
                    match index.search(query, k, ef) {
                        Ok(ids) => {
                            let store = self.inner.store.lock().unwrap();
                            let arr = ids
                                .into_iter()
                                .map(|id| {
                                    let payload =
                                        store.get(&format!("doc:{id}")).and_then(|v| match v {
                                            Value::String(s) => Some(s.clone()),
                                            _ => None,
                                        });
                                    Resp::Array(Some(vec![
                                        Resp::Integer(id as i64),
                                        Resp::Bulk(payload.map(|s| s.into_bytes())),
                                    ]))
                                })
                                .collect();
                            Resp::Array(Some(arr))
                        }
                        Err(e) => Resp::Error(format!("ERR {e}")),
                    }
                } else {
                    Resp::Error("ERR no such index".into())
                }
            }
            "COMPACT" => {
                self.compact();
                Resp::Simple("OK".into())
            }
            _ => Resp::Error("ERR unknown command".into()),
        }
    }

    fn cleanup_key(&self, key: &str) {
        let expired = {
            let mut exp = self.inner.expires.lock().unwrap();
            if let Some(t) = exp.get(key).copied() {
                if Instant::now() >= t {
                    exp.remove(key);
                    true
                } else {
                    false
                }
            } else {
                false
            }
        };
        if expired {
            self.inner.store.lock().unwrap().remove(key);
        }
    }

    fn log(&self, cmd: &[String]) {
        if let Some(file) = &mut *self.inner.aof.lock().unwrap() {
            let arr = Resp::Array(Some(
                cmd.iter()
                    .map(|s| Resp::Bulk(Some(s.clone().into_bytes())))
                    .collect(),
            ));
            let _ = file.write_all(&arr.encode());
            let _ = file.flush();
        }
    }

    fn compact(&self) {
        let path = match &self.inner.aof_path {
            Some(p) => p.clone(),
            None => return,
        };
        let tmp = path.with_extension("tmp");
        let mut file = match File::create(&tmp) {
            Ok(f) => f,
            Err(_) => return,
        };

        {
            let store = self.inner.store.lock().unwrap();
            for (k, v) in store.iter() {
                if let Value::String(s) = v {
                    let cmd = vec!["SET".into(), k.clone(), s.clone()];
                    let arr = Resp::Array(Some(
                        cmd.iter()
                            .map(|s| Resp::Bulk(Some(s.clone().into_bytes())))
                            .collect(),
                    ));
                    let _ = file.write_all(&arr.encode());
                }
            }
        }

        {
            let vecs = self.inner.vectors.lock().unwrap();
            for (name, idx) in vecs.iter() {
                let create = vec![
                    "VEC.CREATE".into(),
                    name.clone(),
                    idx.dim().to_string(),
                    idx.m().to_string(),
                    idx.ef_construction().to_string(),
                ];
                let arr = Resp::Array(Some(
                    create
                        .iter()
                        .map(|s| Resp::Bulk(Some(s.clone().into_bytes())))
                        .collect(),
                ));
                let _ = file.write_all(&arr.encode());

                for (id, vec) in idx.vectors().iter() {
                    let vec_str = vec
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(",");
                    let payload = self
                        .inner
                        .store
                        .lock()
                        .unwrap()
                        .get(&format!("doc:{id}"))
                        .and_then(|v| match v {
                            Value::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .unwrap_or_default();
                    let add = vec![
                        "VEC.ADD".into(),
                        name.clone(),
                        id.to_string(),
                        vec_str,
                        payload,
                    ];
                    let arr = Resp::Array(Some(
                        add.iter()
                            .map(|s| Resp::Bulk(Some(s.clone().into_bytes())))
                            .collect(),
                    ));
                    let _ = file.write_all(&arr.encode());
                }
            }
        }

        let _ = file.flush();
        let _ = std::fs::rename(&tmp, &path);
        if let Ok(f) = OpenOptions::new().append(true).read(true).open(&path) {
            *self.inner.aof.lock().unwrap() = Some(f);
        }

        if let Some(dir) = path.parent() {
            let vecs = self.inner.vectors.lock().unwrap();
            for (name, idx) in vecs.iter() {
                if let Ok(bytes) = idx.dump() {
                    let _ = std::fs::write(dir.join(format!("vec_{}.hnsw", name)), bytes);
                }
            }
        }
    }

    pub fn subscribe(&self, channel: &str) -> broadcast::Receiver<String> {
        let mut map = self.inner.pubsub.lock().unwrap();
        if let Some(tx) = map.get(channel) {
            tx.subscribe()
        } else {
            let (tx, rx) = broadcast::channel(100);
            map.insert(channel.to_string(), tx);
            rx
        }
    }

    pub fn publish(&self, channel: &str, msg: String) -> usize {
        let map = self.inner.pubsub.lock().unwrap();
        if let Some(tx) = map.get(channel) {
            tx.send(msg).unwrap_or(0)
        } else {
            0
        }
    }

    pub async fn listen(self: Arc<Self>, addr: &str) -> std::io::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        loop {
            let (socket, _) = listener.accept().await?;
            let srv = self.clone();
            tokio::spawn(async move {
                let _ = srv.handle_socket(socket).await;
            });
        }
    }

    async fn handle_socket(self: Arc<Self>, mut socket: TcpStream) -> std::io::Result<()> {
        let mut buf = Vec::new();
        loop {
            let mut tmp = [0u8; 1024];
            let n = socket.read(&mut tmp).await?;
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&tmp[..n]);
            while let Ok((msg, used)) = Resp::parse(&buf) {
                let remaining = buf.split_off(used);
                match msg {
                    Resp::Array(Some(arr)) => {
                        let parts: Vec<String> = arr.into_iter().map(resp_to_string).collect();
                        if parts.get(0).map(|s| s.eq_ignore_ascii_case("SUBSCRIBE")) == Some(true) {
                            if let Some(ch) = parts.get(1) {
                                let mut rx = self.subscribe(ch);
                                let ack = Resp::Array(Some(vec![
                                    Resp::Bulk(Some(b"subscribe".to_vec())),
                                    Resp::Bulk(Some(ch.clone().into_bytes())),
                                    Resp::Integer(1),
                                ]));
                                socket.write_all(&ack.encode()).await?;
                                loop {
                                    match rx.recv().await {
                                        Ok(m) => {
                                            let out = Resp::Array(Some(vec![
                                                Resp::Bulk(Some(b"message".to_vec())),
                                                Resp::Bulk(Some(ch.clone().into_bytes())),
                                                Resp::Bulk(Some(m.into_bytes())),
                                            ]));
                                            socket.write_all(&out.encode()).await?;
                                        }
                                        Err(_) => break,
                                    }
                                }
                                return Ok(());
                            }
                        } else {
                            let resp = self.execute(&parts);
                            socket.write_all(&resp.encode()).await?;
                        }
                    }
                    _ => {
                        let err = Resp::Error("ERR protocol".into());
                        socket.write_all(&err.encode()).await?;
                    }
                }
                buf = remaining;
            }
        }
        Ok(())
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn resp_round_trip() {
        let msg = Resp::Array(Some(vec![
            Resp::Simple("OK".into()),
            Resp::Bulk(Some(b"hello".to_vec())),
            Resp::Integer(42),
        ]));
        let enc = msg.encode();
        let parsed = Resp::parse(&enc).unwrap().0;
        assert_eq!(msg, parsed);
    }

    #[test]
    fn ttl_expiry() {
        let r = Redis::new(None);
        r.execute(&vec!["SET".into(), "k".into(), "v".into()]);
        r.execute(&vec!["PEXPIRE".into(), "k".into(), "100".into()]);
        std::thread::sleep(Duration::from_millis(150));
        let resp = r.execute(&vec!["GET".into(), "k".into()]);
        assert_eq!(resp, Resp::Bulk(None));
    }

    #[test]
    fn aof_replay() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("aof.log");
        {
            let r1 = Redis::new(Some(path.clone()));
            r1.execute(&vec!["SET".into(), "foo".into(), "bar".into()]);
        }
        let r2 = Redis::new(Some(path.clone()));
        let resp = r2.execute(&vec!["GET".into(), "foo".into()]);
        assert_eq!(resp, Resp::Bulk(Some(b"bar".to_vec())));
    }

    #[test]
    fn pubsub_delivery() {
        let r = Redis::new(None);
        let mut rx = r.subscribe("chan");
        std::thread::spawn({
            let r = r.clone();
            move || {
                std::thread::sleep(Duration::from_millis(50));
                r.publish("chan", "hello".into());
            }
        });
        let rt = tokio::runtime::Runtime::new().unwrap();
        let msg = rt.block_on(async { rx.recv().await.unwrap() });
        assert_eq!(msg, "hello");
    }

    #[test]
    #[ignore]
    fn vec_index_persistence() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("aof.log");
        {
            let r = Redis::new(Some(path.clone()));
            r.execute(&vec![
                "VEC.CREATE".into(),
                "idx".into(),
                "2".into(),
                "4".into(),
                "10".into(),
            ]);
            r.execute(&vec![
                "VEC.ADD".into(),
                "idx".into(),
                "1".into(),
                "1.0,0.0".into(),
                "doc1".into(),
            ]);
            r.execute(&vec![
                "VEC.ADD".into(),
                "idx".into(),
                "2".into(),
                "0.0,1.0".into(),
                "doc2".into(),
            ]);
            r.execute(&vec!["COMPACT".into()]);
        }

        // restore with snapshot
        let r2 = Redis::new(Some(path.clone()));
        let resp = r2.execute(&vec![
            "VEC.SEARCH".into(),
            "idx".into(),
            "1.0,0.0".into(),
            "1".into(),
            "10".into(),
        ]);
        match resp {
            Resp::Array(Some(results)) => {
                assert_eq!(results.len(), 1);
                if let Resp::Array(Some(inner)) = &results[0] {
                    assert_eq!(inner[0], Resp::Integer(1));
                    assert_eq!(inner[1], Resp::Bulk(Some(b"doc1".to_vec())));
                } else {
                    panic!("unexpected resp");
                }
            }
            _ => panic!("unexpected resp"),
        }

        // remove snapshot and rebuild from AOF
        std::fs::remove_file(dir.path().join("vec_idx.hnsw")).unwrap();
        let r3 = Redis::new(Some(path));
        let resp = r3.execute(&vec![
            "VEC.SEARCH".into(),
            "idx".into(),
            "0.0,1.0".into(),
            "1".into(),
            "10".into(),
        ]);
        match resp {
            Resp::Array(Some(results)) => {
                assert_eq!(results.len(), 1);
                if let Resp::Array(Some(inner)) = &results[0] {
                    assert_eq!(inner[0], Resp::Integer(2));
                    assert_eq!(inner[1], Resp::Bulk(Some(b"doc2".to_vec())));
                } else {
                    panic!("unexpected resp");
                }
            }
            _ => panic!("unexpected resp"),
        }
    }
}
