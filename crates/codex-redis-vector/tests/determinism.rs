use codex_redis_vector::Index;

#[test]
fn hnsw_neighbor_determinism() {
    let data = vec![
        (1, vec![1.0, 0.0]),
        (2, vec![0.9, 0.1]),
        (3, vec![0.0, 1.0]),
    ];
    let mut idx1 = Index::new(2, 16, 200).unwrap();
    for (id, vec) in &data {
        idx1.add(*id, vec.clone()).unwrap();
    }
    let res1 = idx1.search(vec![1.0, 0.0], 2, 200).unwrap();

    let mut idx2 = Index::new(2, 16, 200).unwrap();
    for (id, vec) in &data {
        idx2.add(*id, vec.clone()).unwrap();
    }
    let res2 = idx2.search(vec![1.0, 0.0], 2, 200).unwrap();

    assert_eq!(res1, res2);
}
