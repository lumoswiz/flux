use super::primitives::{BlockNumber, Mps, Price};

pub struct Checkpoint {
    pub block: BlockNumber,
    pub clearing_price: Price,
    pub cumulative_mps: Mps,
    pub prev_block: BlockNumber,
    pub next_block: BlockNumber,
}

impl Checkpoint {
    pub fn remaining_mps(&self) -> Mps {
        self.cumulative_mps.remaining()
    }

    pub fn is_sold_out(&self) -> bool {
        self.cumulative_mps.is_sold_out()
    }

    pub fn is_terminal(&self) -> bool {
        self.next_block == BlockNumber::TAIL_SENTINEL
    }
}
