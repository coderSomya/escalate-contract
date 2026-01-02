use crate::elements::{Card, User};

impl User {
    pub fn new(user_id: String, bio: String) -> Self {
        User {
            user_id,
            bio,
            balance: 100.0,
            cards: Vec::new(),
        }
    }

    pub fn deposit(&mut self, amount: f64) {
        self.balance += amount;
    }

    pub fn add_cards(&mut self, new_cards: Vec<Card>) {
        self.cards.extend(new_cards);
    }
}