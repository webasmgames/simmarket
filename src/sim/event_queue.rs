use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::shared::types::{AgentId, Order, OrderId, StockId};

pub enum OrderAction {
    Submit(Order),
    Cancel(OrderId),
}

pub struct OrderEvent {
    pub intra_tick_offset_us: u32,
    pub agent_id: AgentId,
    pub stock_id: StockId,
    pub action: OrderAction,
}

struct HeapEntry {
    offset: u32,
    seq: u64,
    event: OrderEvent,
}

impl PartialEq for HeapEntry {
    fn eq(&self, other: &Self) -> bool {
        self.offset == other.offset && self.seq == other.seq
    }
}

impl Eq for HeapEntry {}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap by offset; FIFO for equal offsets (lower seq = inserted earlier = pops first)
        other
            .offset
            .cmp(&self.offset)
            .then_with(|| other.seq.cmp(&self.seq))
    }
}

pub struct EventQueue {
    heap: BinaryHeap<HeapEntry>,
    seq: u64,
}

impl EventQueue {
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            seq: 0,
        }
    }

    pub fn push(&mut self, event: OrderEvent) {
        let seq = self.seq;
        self.seq += 1;
        self.heap.push(HeapEntry {
            offset: event.intra_tick_offset_us,
            seq,
            event,
        });
    }

    /// Drain all events sorted ascending by intra_tick_offset_us; FIFO for equal offsets.
    pub fn drain_sorted(&mut self) -> Vec<OrderEvent> {
        let mut result = Vec::with_capacity(self.heap.len());
        while let Some(entry) = self.heap.pop() {
            result.push(entry.event);
        }
        result
    }
}

impl Default for EventQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::types::{Order, OrderType, Side};

    fn dummy_order(id: u64) -> Order {
        Order {
            id,
            agent_id: 1,
            side: Side::Bid,
            order_type: OrderType::Limit,
            price: 10.0,
            quantity: 100,
            filled: 0,
            submitted_at: 0,
            gtc: false,
        }
    }

    fn push_event(q: &mut EventQueue, offset: u32, order_id: u64) {
        q.push(OrderEvent {
            intra_tick_offset_us: offset,
            agent_id: 1,
            stock_id: 0,
            action: OrderAction::Submit(dummy_order(order_id)),
        });
    }

    fn drain_offsets(q: &mut EventQueue) -> Vec<u32> {
        q.drain_sorted()
            .iter()
            .map(|e| e.intra_tick_offset_us)
            .collect()
    }

    fn drain_order_ids(q: &mut EventQueue) -> Vec<u64> {
        q.drain_sorted()
            .into_iter()
            .map(|e| match e.action {
                OrderAction::Submit(o) => o.id,
                OrderAction::Cancel(id) => id,
            })
            .collect()
    }

    #[test]
    fn drains_in_ascending_offset_order() {
        let mut q = EventQueue::new();
        for (offset, id) in [(300, 1), (100, 2), (500, 3), (200, 4), (400, 5)] {
            push_event(&mut q, offset, id);
        }
        assert_eq!(drain_offsets(&mut q), vec![100, 200, 300, 400, 500]);
    }

    #[test]
    fn same_offset_drains_fifo() {
        let mut q = EventQueue::new();
        push_event(&mut q, 100, 10);
        push_event(&mut q, 100, 20);
        push_event(&mut q, 100, 30);
        assert_eq!(drain_order_ids(&mut q), vec![10, 20, 30]);
    }

    #[test]
    fn drain_empties_queue() {
        let mut q = EventQueue::new();
        push_event(&mut q, 50, 1);
        push_event(&mut q, 50, 2);
        let first = q.drain_sorted();
        assert_eq!(first.len(), 2);
        let second = q.drain_sorted();
        assert!(second.is_empty());
    }
}
