use glam::Vec2;
use ndarray::Array2;
use thin_vec::ThinVec;

use super::util::Index;

pub struct NeighborGrid {
    pub data: Array2<ThinVec<u32>>,
    pub unit: f32,
    pub shape: (usize, usize),
}

impl NeighborGrid {
    pub fn new(size: Vec2, unit: f32) -> Self {
        let shape = (size / unit).ceil();
        let shape = (shape.y as usize, shape.x as usize);
        let data = Array2::from_elem(shape, ThinVec::new());

        NeighborGrid { data, unit, shape }
    }

    pub fn update(&mut self, positions: impl IntoIterator<Item = Vec2>) {
        self.data = Array2::from_elem(self.shape, ThinVec::new());

        for (i, pos) in positions.into_iter().enumerate() {
            let ix = (pos / self.unit).as_ivec2() + 1;
            let ix = Index::new(ix.x, ix.y);
            if let Some(neighbors) = self.data.get_mut(ix) {
                if !neighbors.has_capacity() {
                    neighbors.reserve(32);
                }
                neighbors.push(i as u32);
            }
        }
    }
}
