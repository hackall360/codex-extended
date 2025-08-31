use std::collections::HashMap;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use small_world_rs::{
    distance_metric::{CosineDistance, DistanceMetric},
    primitives::vector::Vector,
    world::world::World,
};

/// HNSW vector index backed by `small-world-rs`.
#[derive(Serialize, Deserialize)]
pub struct Index {
    dim: usize,
    m: usize,
    ef_construction: usize,
    vectors: HashMap<u32, Vec<f32>>,
    world: World,
}

impl Index {
    /// Create a new index with the given dimension and parameters.
    pub fn new(dim: usize, m: usize, ef_construction: usize) -> Result<Self> {
        fastrand::seed(1);
        let world = World::new(
            m,
            ef_construction,
            ef_construction,
            DistanceMetric::Cosine(CosineDistance),
        )?;
        Ok(Self {
            dim,
            m,
            ef_construction,
            vectors: HashMap::new(),
            world,
        })
    }

    /// Insert a vector with an id. Existing ids are ignored.
    pub fn add(&mut self, id: u32, vec: Vec<f32>) -> Result<()> {
        if vec.len() != self.dim {
            bail!("dimension mismatch");
        }
        if !self.vectors.contains_key(&id) {
            self.world.insert_vector(id, Vector::new_f32(&vec))?;
        }
        self.vectors.insert(id, vec);
        Ok(())
    }

    /// Search for nearest neighbours.
    pub fn search(&self, query: Vec<f32>, k: usize, ef_search: usize) -> Result<Vec<u32>> {
        if query.len() != self.dim {
            bail!("dimension mismatch");
        }
        self.world.search(&Vector::new_f32(&query), k, ef_search)
    }

    /// Serialize index to bytes.
    pub fn dump(&self) -> Result<Vec<u8>> {
        Ok(bincode::serialize(self)?)
    }

    /// Load index from bytes.
    pub fn load(data: &[u8]) -> Result<Self> {
        Ok(bincode::deserialize(data)?)
    }

    /// Dimension of vectors in this index.
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// M parameter.
    pub fn m(&self) -> usize {
        self.m
    }

    /// efConstruction parameter.
    pub fn ef_construction(&self) -> usize {
        self.ef_construction
    }

    /// All stored vectors for AOF rewrite.
    pub fn vectors(&self) -> &HashMap<u32, Vec<f32>> {
        &self.vectors
    }
}
