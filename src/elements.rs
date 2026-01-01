use serde::{Deserialize, Serialize};
use weil_macros::WeilType;
use weil_rs::runtime::Runtime;

#[derive(Debug, Serialize, Deserialize, WeilType, Clone, Copy, PartialEq, Eq)]
pub enum Card {
    ACE,
    TWO,
    THREE,
    FOUR,
    FIVE,
    SIX,
    SEVEN,
    EIGHT,
    NINE,
    TEN,
    JACK,
    QUEEN,
    KING,
    JOKER
}

impl Card{
    pub fn equivalent(card1: Card, card2: Card) -> bool {
        card1 == Card::JOKER || card2 == Card::JOKER || card1 == card2
    }
}

#[derive(Debug, Serialize, Deserialize, WeilType, Clone)]
pub struct User {
    pub user_id: String,
    pub bio: String,
    pub balance: f64,
    pub cards: Vec<Card>
}

#[derive(Debug, Serialize, Deserialize, WeilType, Clone)]
pub struct Stake {
    pub user_id: String,
    pub cards: Vec<Card>,
}

#[derive(Debug, Serialize, Deserialize, WeilType, Clone)]
pub struct Hand {
    pub hand_id: String,
    pub creator: String,
    pub claimed_card: Card,
    pub is_resolved: bool,
    pub stakes: Vec<Stake>,
}

#[derive(Debug, Serialize, Deserialize, WeilType, Clone)]
pub struct Offer {
    pub offer_id: String,
    pub creator_id: String,
    pub cards: Vec<Card>,
    pub initial_price: f64,
    pub current_bid: Option<f64>,
    pub current_bidder_id: Option<String>,
    pub is_resolved: bool,
}

pub fn get_random_cards(num: u32) -> Vec<Card> {
    let deck = [
        Card::ACE,
        Card::TWO,
        Card::THREE,
        Card::FOUR,
        Card::FIVE,
        Card::SIX,
        Card::SEVEN,
        Card::EIGHT,
        Card::NINE,
        Card::TEN,
        Card::JACK,
        Card::QUEEN,
        Card::KING,
        Card::JOKER,
    ];

    let seed = Runtime::block_height();

    (0..num)
        .map(|i| {
            let idx = ((seed + i as u64) as usize) % deck.len();
            deck[idx]
        })
        .collect()
}

pub fn is_bluff(hand: &Hand) -> bool{
    let claim_card = hand.claimed_card;

    // SAFETY: when u created a hand, you would have immutably 
    // put atleast one initial stake
    let last_stake = hand.stakes.last().unwrap();

    for card in last_stake.cards.iter(){
        if !Card::equivalent(*card, claim_card){
            return true;
        }
    }
    false
}

