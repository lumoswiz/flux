use crate::types::primitives::{BidId, CurrencyAmount, Price};

#[derive(Clone, Debug)]
pub enum Intent {
    SubmitBid {
        max_price: Price,
        amount: CurrencyAmount,
    },
    Exit {
        bid_id: BidId,
    },
    Claim {
        bid_ids: Vec<BidId>,
    },
}
