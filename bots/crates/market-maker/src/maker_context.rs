pub struct MakerContext {
    /// The quote/base mark price, aka the mid price.
    mark_price: f64,
    /// The total size of bids filled in base atoms.
    bid_fills: u64,
    /// The total size of asks filled in base atoms.
    ask_fills: u64,
}

impl MakerContext {
    pub fn mark_price(&self) -> f64 {
        self.mark_price
    }

    pub fn base_inventory(&self) -> i128 {
        self.bid_fills as i128 - self.ask_fills as i128
    }
}
