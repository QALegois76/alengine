// Entity — identifiant unique dans le World.
//
// `generation` invalide les Handle vers des entités supprimées.
// Une entité supprimée incrémente la génération de son slot ;
// toute référence avec l'ancienne génération est considérée invalide.

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Entity {
    pub index: u32,
    pub generation: u32,
}

impl Entity {
    pub fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }
}

// Slot dans le pool d'entités — gère le recyclage des indices.
pub struct EntitySlot {
    pub generation: u32,
    pub alive: bool,
}

// Pool d'entités avec recyclage des indices libérés.
pub struct EntityPool {
    slots: Vec<EntitySlot>,
    free: Vec<u32>,
}

impl EntityPool {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
        }
    }

    pub fn spawn(&mut self) -> Entity {
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.alive = true;
            Entity::new(index, slot.generation)
        } else {
            let index = self.slots.len() as u32;
            self.slots.push(EntitySlot { generation: 0, alive: true });
            Entity::new(index, 0)
        }
    }

    pub fn despawn(&mut self, entity: Entity) -> bool {
        let slot = match self.slots.get_mut(entity.index as usize) {
            Some(s) => s,
            None => return false,
        };
        if !slot.alive || slot.generation != entity.generation {
            return false;
        }
        slot.alive = false;
        slot.generation = slot.generation.wrapping_add(1);
        self.free.push(entity.index);
        true
    }

    pub fn is_alive(&self, entity: Entity) -> bool {
        self.slots
            .get(entity.index as usize)
            .map(|s| s.alive && s.generation == entity.generation)
            .unwrap_or(false)
    }
}
