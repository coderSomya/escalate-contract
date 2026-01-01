
use serde::{Deserialize, Serialize};
use weil_macros::{constructor, mutate, query, smart_contract, WeilType};
use weil_rs::collections::{WeilId, map::WeilMap, vec::WeilVec};
use weil_rs::runtime::Runtime;

mod elements;
use elements::{Card, Hand, Offer, Stake, User, get_random_cards, is_bluff};

mod user;
mod offer;
trait Escalate {
    fn new() -> Result<Self, String>
    where
        Self: Sized;
    async fn register_user(&mut self, bio: String) -> Result<User, String>;
    async fn get_users(&self) -> Vec<User>;
    async fn get_user(&self, id: String) -> Option<User>;
    async fn get_my_cards(&self) -> Result<Vec<Card>, String>;
    async fn start_hand(&mut self, claim: Card, cards: Vec<Card>) -> Result<Hand, String>;
    async fn get_hands(&self) -> Vec<Hand>;
    async fn get_hand(&self, id: String) -> Option<Hand>;
    async fn buy_cards(&mut self, amount: f64) -> Result<Vec<Card>, String>;
    async fn stake(&mut self, hand_id: String, cards: Vec<Card>) -> Result<Hand, String>;
    async fn check(&mut self, hand_id: String) -> Result<bool, String>;
    async fn offer(&mut self, cards: Vec<Card>, amount: f64) -> Result<Offer, String>;
    async fn get_offers(&self) -> Vec<Offer>;
    async fn bid(&mut self, offer_id: String, bid_amout: f64) -> Result<(), String>;
    async fn resolve(&mut self, offer_id: String) -> Result<(), String>;
    async fn withdraw_bid(&mut self, offer_id: String) -> Result<(), String>;
    async fn deposit(&mut self, amount: f64) -> Result<(), String>;
}

const EQUIVALENT_REWARD: f64 = 1.0;
const BLUFF_REWARD: f64 = 1.2;

impl EscalateContractState {
    fn remove_cards_from_inventory(
        inventory: &mut Vec<Card>,
        cards: &[Card],
    ) -> Result<(), String> {
        for card in cards {
            if let Some(idx) = inventory.iter().position(|c| c == card) {
                inventory.remove(idx);
            } else {
                return Err(format!("not enough {:?} cards to stake", card));
            }
        }
        Ok(())
    }

    fn reward_stakers(&mut self, stakes: &[Stake], include_last: bool, claimed: Card) {
        if stakes.is_empty() {
            return;
        }

        let upto = if include_last {
            stakes.len()
        } else {
            stakes.len().saturating_sub(1)
        };

        for stake in stakes.iter().take(upto) {
            if let Some(mut staker) = self.users.get(&stake.user_id) {
                for card in &stake.cards {
                    let reward = if Card::equivalent(*card, claimed) {
                        EQUIVALENT_REWARD
                    } else {
                        BLUFF_REWARD
                    };
                    staker.balance += reward;
                }
                self.users.insert(stake.user_id.clone(), staker);
            }
        }
    }

    fn next_hand_id(&mut self) -> String {
        self.hand_counter += 1;
        self.hand_counter.to_string()
    }

    fn next_offer_id(&mut self) -> String {
        self.offer_counter += 1;
        self.offer_counter.to_string()
    }

    fn mask_hand_for_view(hand: &Hand) -> Hand {
        let mut masked = hand.clone();
        masked.stakes = hand
            .stakes
            .iter()
            .map(|s| Stake {
                user_id: s.user_id.clone(),
                // preserve count and order, hide card identities
                cards: vec![Card::JOKER; s.cards.len()],
            })
            .collect();
        masked
    }
}

#[derive(Serialize, Deserialize, WeilType)]
pub struct EscalateContractState {
    users: WeilMap<String, User>,
    user_ids: WeilVec<String>,
    hands: WeilMap<String, Hand>,
    hand_ids: WeilVec<String>,
    offers: WeilMap<String, Offer>,
    offer_ids: WeilVec<String>,
    hand_counter: u64,
    offer_counter: u64,
}

#[smart_contract]
impl Escalate for EscalateContractState {
    #[constructor]
    fn new() -> Result<Self, String>
    where
        Self: Sized,
    {
        Ok(
            Self {
                users: WeilMap::new(WeilId(1)),
                user_ids: WeilVec::new(WeilId(2)),
                hands: WeilMap::new(WeilId(3)),
                hand_ids: WeilVec::new(WeilId(4)),
                offers: WeilMap::new(WeilId(5)),
                offer_ids: WeilVec::new(WeilId(6)),
                hand_counter: 0,
                offer_counter: 0,
            }
        )
    }


    #[mutate]
    async fn register_user(&mut self, bio: String) -> Result<User, String> {
        let sender = Runtime::sender();

        if let Some(mut existing) = self.users.get(&sender) {
            existing.bio = bio;
            self.users.insert(sender.clone(), existing.clone());
            return Ok(existing);
        }

        let user = User::new(sender.clone(), bio);
        self.users.insert(sender.clone(), user.clone());
        self.user_ids.push(sender);
        Ok(user)
    }

    #[query]
    async fn get_users(&self) -> Vec<User> {
        self.user_ids
            .iter()
            .filter_map(|id| self.users.get(&id))
            .collect()
    }

    #[query]
    async fn get_user(&self, id: String) -> Option<User> {
        self.users.get(&id)
    }

    #[query]
    async fn get_my_cards(&self) -> Result<Vec<Card>, String> {
        let sender = Runtime::sender();
        match self.users.get(&sender) {
            Some(u) => Ok(u.cards.clone()),
            None => Err("user not registered".to_string()),
        }
    }

    #[mutate]
    async fn start_hand(&mut self, claim: Card, cards: Vec<Card>) -> Result<Hand, String> {
        let sender = Runtime::sender();
        let mut user = self
            .users
            .get(&sender)
            .ok_or_else(|| "user must register before starting a hand".to_string())?;

        EscalateContractState::remove_cards_from_inventory(&mut user.cards, &cards)
            .map_err(|e| e.to_string())?;

        let hand_id = self.next_hand_id();
        let stake = Stake {
            user_id: sender.clone(),
            cards: cards.clone(),
        };
        let hand = Hand {
            hand_id: hand_id.clone(),
            creator: sender.clone(),
            claimed_card: claim,
            is_resolved: false,
            stakes: vec![stake],
        };

        self.users.insert(sender.clone(), user);
        self.hands.insert(hand_id.clone(), hand.clone());
        self.hand_ids.push(hand_id);

        Ok(hand)
    }

    #[query]
    async fn get_hands(&self) -> Vec<Hand> {
        self.hand_ids
            .iter()
            .filter_map(|id| self.hands.get(&id).map(|h| EscalateContractState::mask_hand_for_view(&h)))
            .collect()
    }

    #[query]
    async fn get_hand(&self, id: String) -> Option<Hand> {
        self.hands
            .get(&id)
            .map(|h| EscalateContractState::mask_hand_for_view(&h))
    }

    #[mutate]
    async fn buy_cards(&mut self, amount: f64) -> Result<Vec<Card>, String> {
        let sender = Runtime::sender();
        let mut user = self
            .users
            .get(&sender)
            .ok_or_else(|| "user must register before buying cards".to_string())?;

        let spend = amount.floor();
        if spend <= 0.0 {
            return Ok(Vec::new());
        }
        if user.balance < spend {
            return Err("insufficient balance".to_string());
        }

        let count = spend as u32;
        let new_cards = get_random_cards(count);
        user.balance -= spend;
        user.cards.extend(new_cards.clone());

        self.users.insert(sender, user);
        Ok(new_cards)
    }

    #[mutate]
    async fn stake(&mut self, hand_id: String, cards: Vec<Card>) -> Result<Hand, String> {
        let sender = Runtime::sender();
        let mut user = self
            .users
            .get(&sender)
            .ok_or_else(|| "user must register before staking".to_string())?;

        let mut hand = self
            .hands
            .get(&hand_id)
            .ok_or_else(|| "hand not found for staking".to_string())?;

        if hand.is_resolved {
            return Err("cannot stake on a resolved hand".to_string());
        }

        EscalateContractState::remove_cards_from_inventory(&mut user.cards, &cards)
            .map_err(|e| e.to_string())?;

        hand.stakes.push(Stake {
            user_id: sender.clone(),
            cards: cards.clone(),
        });

        self.users.insert(sender.clone(), user);
        self.hands.insert(hand_id.clone(), hand.clone());
        Ok(hand)
    }

    #[mutate]
    async fn check(&mut self, hand_id: String) -> Result<bool, String> {
        let checker_id = Runtime::sender();
        let mut checker = self
            .users
            .get(&checker_id)
            .ok_or_else(|| "user must register before checking".to_string())?;

        let mut hand = self
            .hands
            .get(&hand_id)
            .ok_or_else(|| "hand not found for check".to_string())?;
        if hand.is_resolved {
            return Err("hand already resolved".to_string());
        }

        if hand.stakes.is_empty() {
            return Err("no stakes to check".to_string());
        }

        let last_stake_len = hand.stakes.last().unwrap().cards.len() as f64;
        let bluff_detected = is_bluff(&hand);

        if bluff_detected {
            checker.balance += last_stake_len;
            self.reward_stakers(&hand.stakes, false, hand.claimed_card);
        } else {
            checker.balance -= last_stake_len;
            self.reward_stakers(&hand.stakes, true, hand.claimed_card);
        }

        hand.is_resolved = true;

        self.users.insert(checker_id, checker);
        self.hands.insert(hand_id, hand);

        Ok(bluff_detected)
    }

    #[mutate]
    async fn offer(&mut self, cards: Vec<Card>, amount: f64) -> Result<Offer, String> {
        let sender = Runtime::sender();
        let mut user = self
            .users
            .get(&sender)
            .ok_or_else(|| "user must register before offering cards".to_string())?;

        EscalateContractState::remove_cards_from_inventory(&mut user.cards, &cards)
            .map_err(|e| e.to_string())?;

        let offer_id = self.next_offer_id();
        let offer = Offer {
            offer_id: offer_id.clone(),
            creator_id: sender.clone(),
            cards,
            initial_price: amount,
            current_bid: None,
            current_bidder_id: None,
            is_resolved: false,
        };

        self.users.insert(sender, user);
        self.offers.insert(offer_id.clone(), offer.clone());
        self.offer_ids.push(offer_id);

        Ok(offer)
    }

    #[query]
    async fn get_offers(&self) -> Vec<Offer> {
        self.offer_ids
            .iter()
            .filter_map(|id| self.offers.get(&id))
            .collect()
    }

    #[mutate]
    async fn bid(&mut self, offer_id: String, bid_amout: f64) -> Result<(), String> {
        let bidder_id = Runtime::sender();
        let mut bidder = self
            .users
            .get(&bidder_id)
            .ok_or_else(|| "user must register before bidding".to_string())?;

        let mut offer = self
            .offers
            .get(&offer_id)
            .ok_or_else(|| "offer not found".to_string())?;

        if offer.is_resolved {
            return Err("cannot bid on resolved offer".to_string());
        }

        if offer.creator_id == bidder_id {
            return Err("creator cannot bid on own offer".to_string());
        }

        let min_bid = offer.current_bid.unwrap_or(offer.initial_price);
        if bid_amout <= min_bid {
            return Err("bid must be higher than current bid or initial price".to_string());
        }

        // refund previous highest bidder, if any
        if let (Some(prev_amount), Some(prev_bidder_id)) =
            (offer.current_bid, offer.current_bidder_id.clone())
        {
            if prev_bidder_id == bidder_id {
                bidder.balance += prev_amount;
            } else if let Some(mut prev_bidder) = self.users.get(&prev_bidder_id) {
                prev_bidder.balance += prev_amount;
                self.users.insert(prev_bidder_id, prev_bidder);
            }
        }

        if bidder.balance < bid_amout {
            return Err("insufficient balance for bid".to_string());
        }

        bidder.balance -= bid_amout;

        offer.current_bid = Some(bid_amout);
        offer.current_bidder_id = Some(bidder_id.clone());

        self.users.insert(bidder_id, bidder);
        self.offers.insert(offer_id, offer);
        Ok(())
    }

    #[mutate]
    async fn resolve(&mut self, offer_id: String) -> Result<(), String> {
        let sender = Runtime::sender();
        let mut offer = self
            .offers
            .get(&offer_id)
            .ok_or_else(|| "offer not found".to_string())?;

        if offer.creator_id != sender {
            return Err("only creator can resolve offer".to_string());
        }

        if offer.is_resolved {
            return Ok(());
        }

        if let (Some(bid_amount), Some(bidder_id)) =
            (offer.current_bid, offer.current_bidder_id.clone())
        {
            let mut bidder = self
                .users
                .get(&bidder_id)
                .ok_or_else(|| "bidder not registered anymore".to_string())?;
            let mut creator = self
                .users
                .get(&sender)
                .ok_or_else(|| "creator not registered anymore".to_string())?;

            creator.balance += bid_amount;
            bidder.cards.extend(offer.cards.clone());

            offer.is_resolved = true;

            self.users.insert(bidder_id, bidder);
            self.users.insert(sender, creator);
        } else {
            // no bids: return cards to creator
            if let Some(mut creator) = self.users.get(&sender) {
                creator.cards.extend(offer.cards.clone());
                self.users.insert(sender, creator);
            }
            offer.is_resolved = true;
        }

        self.offers.insert(offer_id, offer);
        Ok(())
    }

    #[mutate]
    async fn withdraw_bid(&mut self, offer_id: String) -> Result<(), String> {
        let sender = Runtime::sender();
        let mut offer = self
            .offers
            .get(&offer_id)
            .ok_or_else(|| "offer not found".to_string())?;

        if offer.current_bidder_id.as_deref() != Some(&sender) {
            return Err("only current bidder can withdraw bid".to_string());
        }

        if let Some(amount) = offer.current_bid {
            if let Some(mut bidder) = self.users.get(&sender) {
                bidder.balance += amount;
                self.users.insert(sender.clone(), bidder);
            }
        }

        offer.current_bid = None;
        offer.current_bidder_id = None;

        self.offers.insert(offer_id, offer);
        Ok(())
    }

    #[mutate]
    async fn deposit(&mut self, amount: f64) -> Result<(), String> {
        let sender = Runtime::sender();
        let mut user = self
            .users
            .get(&sender)
            .unwrap_or_else(|| User::new(sender.clone(), "".to_string()));

        if amount <= 0.0 {
            return Err("deposit amount must be positive".to_string());
        }

        user.balance += amount;
        if self.users.get(&sender).is_none() {
            self.user_ids.push(sender.clone());
        }
        self.users.insert(sender, user);
        Ok(())
    }
}

