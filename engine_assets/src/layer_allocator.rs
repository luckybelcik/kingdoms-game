#[derive(Debug)]
pub struct LayerAllocator {
    next_index: u32,
    max_capacity: u32,
    free_slots: Vec<u32>,
}

impl LayerAllocator {
    pub fn new(initial_count: u32, padding: u32) -> Self {
        Self {
            next_index: initial_count,
            max_capacity: initial_count + padding,
            free_slots: Vec::new(),
        }
    }

    pub fn allocate(&mut self) -> Option<u32> {
        if let Some(slot) = self.free_slots.pop() {
            return Some(slot);
        }
        if self.next_index < self.max_capacity {
            let idx = self.next_index;
            self.next_index += 1;
            Some(idx)
        } else {
            None // out of memory, restart the game
        }
    }

    pub fn deallocate(&mut self, index: u32) {
        self.free_slots.push(index);
    }

    pub fn max_capacity(&self) -> u32 {
        self.max_capacity
    }

    pub fn estimate_heap(&self) -> usize {
        self.free_slots.capacity() * size_of::<u32>() + size_of::<u32>() + size_of::<u32>()
    }
}
