//! Frame collision geometry separated from mutable persistent actors.
use super::{
    dynamic::{CollisionProperties, MovingCell},
    hit::Shot,
    registry::Registry,
    runtime::BodyRegistration,
    subtractive::DynamicCell,
};
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DynamicSlot {
    Actor(usize),
    Scene(usize),
}
#[derive(Clone)]
pub struct ActorFrame {
    pub cells: Vec<(u32, MovingCell)>,
    pub scene: Vec<super::scene_cells::SceneCell>,
    collided: Vec<u32>,
    order: Vec<DynamicSlot>,
    pub bodies: BTreeMap<u32, BodyRegistration>,
    pub particles: BTreeMap<usize, Shot>,
}
impl ActorFrame {
    pub fn apply_weapon_hits<F>(
        &mut self,
        registry: &Registry,
        damage: &[i32; 8],
        switches: &mut [u8; 256],
        find_cell: F,
    ) -> Result<(), &'static str>
    where
        F: FnMut([f32; 4]) -> Option<usize>,
    {
        self.dispatch_damage(registry, None, damage, switches, find_cell)
            .map(|_| ())
    }
    pub fn apply_damage<F>(
        &mut self,
        registry: &Registry,
        inventory: &mut crate::game::Inventory,
        damage: &[i32; 8],
        switches: &mut [u8; 256],
        find_cell: F,
    ) -> Result<Vec<crate::game::player::Sound>, &'static str>
    where
        F: FnMut([f32; 4]) -> Option<usize>,
    {
        self.dispatch_damage(registry, Some(inventory), damage, switches, find_cell)
    }
    /// Released zone tail: player first, then each distinct dynamic cell in roster order.
    fn dispatch_damage<F>(
        &mut self,
        registry: &Registry,
        inventory: Option<&mut crate::game::Inventory>,
        damage: &[i32; 8],
        switches: &mut [u8; 256],
        mut find_cell: F,
    ) -> Result<Vec<crate::game::player::Sound>, &'static str>
    where
        F: FnMut([f32; 4]) -> Option<usize>,
    {
        if self.collided.len() != self.cells.len() + self.scene.len() {
            return Err("released collision mask count differs");
        }
        let player_mask = self
            .scene
            .iter()
            .filter(|cell| cell.player)
            .fold(0, |mask, cell| {
                mask | cell.interactions.enter | cell.interactions.collided
            });
        let mut targets = Vec::<(u32, u32)>::new();
        let mut indices = BTreeMap::<(bool, usize), usize>::new();
        for slot in &self.order {
            let (identity, token, mask) = match *slot {
                DynamicSlot::Actor(index) => {
                    let token = self.cells[index].0;
                    ((true, token as usize), Some(token), self.collided[index])
                }
                DynamicSlot::Scene(index) => {
                    let cell = &self.scene[index];
                    (
                        (false, std::rc::Rc::as_ptr(&cell.source) as usize),
                        cell.actor,
                        cell.interactions.enter | cell.interactions.collided,
                    )
                }
            };
            let Some(token) = token else { continue };
            if let Some(&index) = indices.get(&identity) {
                targets[index].1 |= mask;
            } else {
                indices.insert(identity, targets.len());
                targets.push((token, mask));
            }
        }
        targets.retain(|(_, mask)| mask & 0xff00 != 0);
        let mut handles = BTreeMap::new();
        for &(token, _) in &targets {
            if let std::collections::btree_map::Entry::Vacant(entry) = handles.entry(token) {
                entry.insert(
                    registry
                        .get(token)
                        .ok_or("released collider identity missing")?,
                );
            }
        }
        let mut actors = handles
            .iter()
            .map(|(&token, actor)| {
                actor
                    .try_borrow_mut()
                    .map(|actor| (token, actor))
                    .map_err(|_| "released actor already borrowed")
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        for actor in actors.values() {
            if actor.initial.life > 0 && actor.initial.copied.parameters[11] as i32 > 255 {
                return Err("monster kill switch out of range");
            }
        }
        let consume_player = inventory.is_some();
        let mut scene_handles = BTreeMap::new();
        for cell in &self.scene {
            if consume_player || !cell.player {
                scene_handles
                    .entry(std::rc::Rc::as_ptr(&cell.source) as usize)
                    .or_insert_with(|| std::rc::Rc::clone(&cell.source));
            }
        }
        let mut scene_states = scene_handles
            .values()
            .map(|source| {
                source
                    .try_borrow_mut()
                    .map_err(|_| "scene dynamic cell already borrowed")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut sounds = Vec::new();
        if let Some(inventory) = inventory {
            for (weapon, &hits) in damage.iter().enumerate() {
                if player_mask & (1 << (weapon + 8)) != 0 {
                    sounds.extend(inventory.hit(hits));
                }
            }
        }
        for (token, mask) in targets {
            let actor = actors
                .get_mut(&token)
                .ok_or("released collider identity missing")?;
            for (weapon, &hits) in damage.iter().enumerate() {
                if mask & (1 << (weapon + 8)) != 0 {
                    actor.hit(hits, switches, &mut find_cell)?;
                }
            }
        }
        for source in &mut scene_states {
            source.cell.interactions = Default::default();
        }
        self.collided[..self.cells.len()].fill(0);
        for (index, cell) in self.scene.iter_mut().enumerate() {
            if consume_player || !cell.player {
                cell.interactions = Default::default();
                self.collided[self.cells.len() + index] = 0;
            }
        }
        Ok(sounds)
    }
    pub fn capture(
        registry: &Registry,
        colliders: &[u32],
        first_particle: usize,
    ) -> Result<Self, &'static str> {
        let mut cells = Vec::with_capacity(colliders.len());
        for &token in colliders {
            let actor = registry
                .get(token)
                .ok_or("released collider identity missing")?;
            let actor = actor
                .try_borrow()
                .map_err(|_| "released actor already borrowed")?;
            cells.push((token, actor.hit_cell.clone()));
        }
        let mut bodies = BTreeMap::new();
        let mut particles = BTreeMap::new();
        registry.visit_active(|token, actor| {
            if bodies.contains_key(&token) {
                return Ok(());
            }
            let base = (token as usize - 1)
                .checked_mul(5)
                .and_then(|n| first_particle.checked_add(n))
                .ok_or("released particle identity exhausted")?;
            base.checked_add(4)
                .ok_or("released particle identity exhausted")?;
            let ids = std::array::from_fn(|i| base + i);
            for (particle, id) in actor.body.particles.iter().zip(ids) {
                particles.insert(
                    id,
                    Shot {
                        kind: particle.flags[3],
                        event: None,
                    },
                );
            }
            bodies.insert(
                token,
                BodyRegistration {
                    particles: ids,
                    skip_cell: cells.iter().position(|(id, _)| *id == token),
                },
            );
            Ok(())
        })?;
        let collided = vec![0; cells.len()];
        let order = (0..cells.len()).map(DynamicSlot::Actor).collect();
        Ok(Self {
            collided,
            order,
            cells,
            scene: Vec::new(),
            bodies,
            particles,
        })
    }
    pub fn attach_scene(
        &mut self,
        scene: Vec<super::scene_cells::SceneCell>,
    ) -> Result<(), &'static str> {
        if !self.scene.is_empty() {
            return Err("released scene cells already attached");
        }
        self.collided
            .extend(scene.iter().map(|cell| cell.interactions.collided));
        self.order.extend((0..scene.len()).map(DynamicSlot::Scene));
        self.scene = scene;
        Ok(())
    }
    pub fn set_order(&mut self, order: &[DynamicSlot]) -> Result<(), &'static str> {
        let unique: std::collections::BTreeSet<_> = order.iter().copied().collect();
        if order.len() != self.cells.len() + self.scene.len()
            || unique.len() != order.len()
            || order.iter().any(|slot| match slot {
                DynamicSlot::Actor(index) => *index >= self.cells.len(),
                DynamicSlot::Scene(index) => *index >= self.scene.len(),
            })
        {
            return Err("released dynamic order differs from registry");
        }
        self.order = order.to_vec();
        for (token, body) in &mut self.bodies {
            body.skip_cell = self.order.iter().position(|slot| match slot {
                DynamicSlot::Actor(index) => self.cells[*index].0 == *token,
                DynamicSlot::Scene(_) => false,
            });
        }
        Ok(())
    }
    pub fn save_collision_masks(&mut self, masks: &[u32]) -> Result<(), &'static str> {
        if masks.len() != self.cells.len() + self.scene.len() {
            return Err("released collision mask count differs");
        }
        for (slot, &mask) in self.order.iter().zip(masks) {
            let index = match slot {
                DynamicSlot::Actor(index) => *index,
                DynamicSlot::Scene(index) => {
                    self.scene[*index].interactions.collided = mask;
                    self.cells.len() + index
                }
            };
            self.collided[index] = mask;
        }
        Ok(())
    }
    /// Commit accepted geometry only after all actor identities can be borrowed.
    pub fn publish_geometry(&self, registry: &Registry) -> Result<(), &'static str> {
        let mut latest = BTreeMap::new();
        for (index, (token, _)) in self.cells.iter().enumerate() {
            latest.insert(*token, index);
        }
        let handles = latest
            .keys()
            .map(|&token| {
                registry
                    .get(token)
                    .ok_or("released collider identity missing")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut actors = handles
            .iter()
            .map(|handle| {
                handle
                    .try_borrow_mut()
                    .map_err(|_| "released actor already borrowed")
            })
            .collect::<Result<Vec<_>, _>>()?;
        for (actor, index) in actors.iter_mut().zip(latest.values()) {
            actor.hit_cell = self.cells[*index].1.clone();
        }
        for scene in &self.scene {
            scene.publish()?;
        }
        Ok(())
    }

    pub fn dynamic_views(&self) -> Result<Vec<DynamicCell<'_>>, &'static str> {
        self.order
            .iter()
            .map(|slot| match *slot {
                DynamicSlot::Actor(index) => {
                    let (token, cell) = &self.cells[index];
                    cell.collision_view(CollisionProperties {
                        only_for_shot: false,
                        player: false,
                        part_box: Some(*token as usize),
                        collided: self.collided[index],
                    })
                }
                DynamicSlot::Scene(index) => self.scene[index].collision_view(),
            })
            .collect()
    }
}

impl ActorFrame {
    /// Rebuild borrowed cast views after each accepted geometry update.
    pub fn reconcile_all(
        &mut self,
        runtime: &mut super::runtime::CollisionRuntime<'_, '_>,
        registry: &Registry,
    ) -> Result<(), &'static str> {
        self.reconcile_all_with_bodies(runtime, registry, &mut [])
    }
    pub fn reconcile_all_with_bodies(
        &mut self,
        runtime: &mut super::runtime::CollisionRuntime<'_, '_>,
        registry: &Registry,
        prefix: &mut [(super::body::Body, BodyRegistration)],
    ) -> Result<(), &'static str> {
        if runtime.candidates.dynamic_cells.len() != self.cells.len() + self.scene.len() {
            return Err("released dynamic registry count differs");
        }
        let initial: Vec<_> = runtime
            .candidates
            .dynamic_cells
            .iter()
            .map(|cell| cell.collided)
            .collect();
        self.save_collision_masks(&initial)?;
        for index in 0..self.order.len() {
            let slot = self.order[index];
            let mut proposed = match slot {
                DynamicSlot::Actor(id) => self.cells[id].1.clone(),
                DynamicSlot::Scene(id) => self.scene[id].cell.clone(),
            };
            let masks: Vec<u32> = {
                let mut views = self.dynamic_views()?;
                let mut zones = runtime.zones.to_vec();
                let mut probe = super::runtime::CollisionRuntime {
                    adds: runtime.adds,
                    candidates: super::subtractive::CandidateWorld {
                        static_cells: runtime.candidates.static_cells,
                        lists: runtime.candidates.lists,
                        dynamic_cells: &mut views,
                    },
                    zones: &mut zones,
                    zone_lists: runtime.zone_lists,
                    hits: runtime.hits,
                    particle_id: runtime.particle_id,
                    skip_cell: runtime.skip_cell,
                    monster_mode: runtime.monster_mode,
                    interrupted: runtime.interrupted,
                    error: runtime.error,
                };
                let result = probe.reconcile_body_roster(
                    index,
                    &mut proposed,
                    registry,
                    &self.bodies,
                    prefix,
                );
                runtime.interrupted = probe.interrupted;
                runtime.error = probe.error;
                for (target, source) in runtime.zones.iter_mut().zip(probe.zones.iter()) {
                    target.enter = source.enter;
                    target.leave = source.leave;
                    target.collided = source.collided;
                }
                result?;
                probe
                    .candidates
                    .dynamic_cells
                    .iter()
                    .map(|cell| cell.collided)
                    .collect()
            };
            self.save_collision_masks(&masks)?;
            match slot {
                DynamicSlot::Actor(id) => self.cells[id].1 = proposed,
                DynamicSlot::Scene(id) => self.scene[id].cell = proposed,
            }
        }
        self.publish_geometry(registry)
    }
}
