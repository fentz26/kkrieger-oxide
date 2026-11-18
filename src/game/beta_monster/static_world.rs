//! Owned released static hulls and stable registry indices.
use super::{hull::Hull, subtractive::StaticCell, world_cast::AdditiveCell};
use crate::collision::{Cell, Interactions, connectivity};

pub struct StaticWorld {
    pub hulls: Vec<Hull>,
    /// Additive registry index to source cell index.
    pub adds: Vec<usize>,
    pub neighbors: Vec<Vec<usize>>,
    pub subtractive_lists: Vec<Vec<usize>>,
    pub zone_lists: Vec<Vec<usize>>,
}
impl StaticWorld {
    pub fn from_document(
        document: &crate::kx::Document<'_>,
        root: usize,
    ) -> Result<Self, crate::collision::world::BuildError> {
        if document.layout != crate::kx::Layout::Beta2004 {
            return Err(crate::collision::world::BuildError::World(
                "released collision world requires beta document",
            ));
        }
        let scene = crate::collision::world::World::from_document(document, root)?;
        Ok(Self::new(
            scene
                .entries
                .iter()
                .map(|entry| (entry.cell.vertices, entry.cell.mode)),
        ))
    }

    pub fn new(cells: impl IntoIterator<Item = ([[f32; 4]; 8], u32)>) -> Self {
        let mut geometry = Vec::new();
        let hulls: Vec<_> = cells
            .into_iter()
            .map(|(vertices, mode)| {
                let hull = Hull::from_vertices(vertices, mode);
                geometry.push(Cell {
                    vertices,
                    planes: hull.planes[..hull.count].to_vec(),
                    mode: hull.mode,
                    only_for_shot: hull.only_for_shot,
                    center: hull.center,
                    bounds: hull.bounds,
                    interactions: Interactions::default(),
                });
                hull
            })
            .collect();
        let links = connectivity::connect(&geometry);
        let adds: Vec<_> = hulls
            .iter()
            .enumerate()
            .filter_map(|(id, hull)| (hull.mode & 3 == 0).then_some(id))
            .collect();
        let mut additive_index = vec![None; hulls.len()];
        for (index, &id) in adds.iter().enumerate() {
            additive_index[id] = Some(index);
        }
        let neighbors = adds
            .iter()
            .map(|&id| {
                links.neighbors[id]
                    .adds
                    .iter()
                    .map(|&other| additive_index[other].unwrap())
                    .collect()
            })
            .collect();
        let subtractive_lists = adds
            .iter()
            .map(|&id| links.neighbors[id].subs.clone())
            .collect();
        let zone_lists = adds
            .iter()
            .map(|&id| links.neighbors[id].zones.clone())
            .collect();
        Self {
            hulls,
            adds,
            neighbors,
            subtractive_lists,
            zone_lists,
        }
    }
    pub fn additive_views(&self) -> Vec<AdditiveCell<'_>> {
        self.adds
            .iter()
            .enumerate()
            .map(|(index, &id)| {
                let hull = &self.hulls[id];
                AdditiveCell {
                    planes: &hull.planes[..hull.count],
                    neighbors: &self.neighbors[index],
                    only_for_shot: hull.only_for_shot,
                }
            })
            .collect()
    }
    /// Source indices remain valid in subtractive lists.
    pub fn static_views(&self) -> Vec<StaticCell<'_>> {
        self.hulls
            .iter()
            .map(|hull| StaticCell {
                planes: &hull.planes[..hull.count],
                mode: hull.mode,
                only_for_shot: hull.only_for_shot,
            })
            .collect()
    }
}
