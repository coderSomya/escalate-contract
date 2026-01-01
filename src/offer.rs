use crate::elements::{Card, Offer};

impl Offer {
    pub fn new(offer_id: String, creator_id: String, cards: Vec<Card>, amount: f64) -> Self {
        Offer {
            offer_id,
            creator_id,
            cards,
            initial_price: amount,
            current_bid: None,
            current_bidder_id: None,
            is_resolved: false,
        }
    }
}