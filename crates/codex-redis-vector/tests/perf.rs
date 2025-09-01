use codex_redis_vector::Index;
use std::time::Instant;

#[test]
#[ignore]
fn hnsw_query_p95_latency() {
    let dim = 16;
    let mut idx = Index::new(dim, 16, 200).unwrap();
    let mut queries = Vec::new();
    for id in 0..100_000u32 {
        let vec: Vec<f32> = (0..dim).map(|_| fastrand::f32()).collect();
        if id < 100 {
            queries.push(vec.clone());
        }
        idx.add(id, vec).unwrap();
    }
    let mut times = Vec::new();
    for q in &queries {
        let start = Instant::now();
        let _ = idx.search(q.clone(), 10, 200).unwrap();
        times.push(start.elapsed());
    }
    times.sort();
    let idx95 = (times.len() as f64 * 0.95).ceil() as usize - 1;
    let p95 = times[idx95];
    println!("p95 query latency: {:?}", p95);
}
