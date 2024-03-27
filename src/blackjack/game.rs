use mongodb::bson::uuid::Uuid;
use strum::IntoEnumIterator;

use crate::card::{Card, CardSuit, CardValue};

use rand::{seq::SliceRandom, thread_rng};

// Number of decks used by table.
const NUM_OF_DECKS: u8 = 3;

struct PlayerCard {
    pub card: Card,
    pub visible: bool,
}

struct Player {
    id: Uuid,
    hand: Vec<PlayerCard>,
}

struct Table {
    id: Uuid,
    players: Vec<Player>,
    deck: Vec<Card>,
    dealer: Vec<PlayerCard>,
}

pub struct Blackjack {
    tables: Vec<Table>,
}

impl Blackjack {
    pub fn create_game() -> Blackjack {
        let tables: Vec<Table> = Vec::new();
        Blackjack { tables }
    }
    pub fn get_table(&self, id: Uuid) -> Option<usize> {
        self.tables.iter().position(|x: &Table| x.id == id)
    }
    pub fn remove_table(&mut self, id: Uuid) -> bool {
        let index = self.get_table(id);
        match index {
            Some(num) => {
                self.tables.remove(num);
                true
            }
            None => false,
        }
    }
    pub fn add_table(&mut self) -> Uuid {
        self.tables.push(Table {
            id: Uuid::new(),
            players: Vec::new(),
            deck: Blackjack::create_deck(),
            dealer: Vec::new(),
        });
        self.tables.last().unwrap().id
    }

    pub fn create_deck() -> Vec<Card> {
        let mut deck: Vec<Card> = Vec::new();
        for suit in CardSuit::iter() {
            for value in CardValue::iter() {
                let card = Card { suit, value };
                for _num in 1..=NUM_OF_DECKS {
                    deck.push(card);
                }
            }
        }
        let mut rng = thread_rng();
        deck.as_mut_slice().shuffle(&mut rng);
        deck
    }
}

impl Table {
    pub fn add_player(&mut self, id: Uuid) {
        let player = Player {
            id,
            hand: Vec::new(),
        };
        self.players.push(player);
    }

    pub fn get_player(&self, id: Uuid) -> Option<usize> {
        self.players.iter().position(|x: &Player| x.id == id)
    }

    pub fn remove_player(&mut self, id: Uuid) -> bool {
        let index = self.get_player(id);
        match index {
            Some(num) => {
                self.players.remove(num);
                true
            }
            None => false,
        }
    }

    pub fn add_card(&mut self, id: Uuid) -> bool {
        let index = self.get_player(id);
        match index {
            Some(num) => {
                self.players[num].hand.push(PlayerCard {
                    card: self.deck.pop().unwrap(),
                    visible: true,
                });
                true
            }
            None => false,
        }
    }

    pub fn add_card_dealer(&mut self) {
        self.dealer.push(PlayerCard {
            card: self.deck.pop().unwrap(),
            visible: false,
        });
    }

    pub fn check_winner(&self, id: Uuid) -> bool {
        let index = self.get_player(id);
        match index {
            Some(num) => {
                // ( total with ace = 1, ( if ace present , total with one ace = 11) )
                let mut player_total = (0, (false, 0));
                for player_card in self.players[num].hand.iter() {
                    let PlayerCard {
                        card,
                        visible: _visible,
                    } = player_card;
                    player_total.0 += card.value as i32 + 1;
                    match card.value as i32 {
                        0 => {
                            if !player_total.1 .0 {
                                player_total.1 .1 += card.value as i32 + 11;
                                player_total.1 .0 = true;
                            } else {
                                player_total.1 .1 += card.value as i32 + 1;
                            }
                        }
                        _ => player_total.1 .1 += card.value as i32 + 1,
                    }
                }
                // ( total with ace = 1, ( if ace present , total with one ace = 11) )
                let mut dealer_total = (0, (false, 0));
                for player_card in self.dealer.as_slice() {
                    let PlayerCard {
                        card,
                        visible: _visible,
                    } = player_card;
                    dealer_total.0 += card.value as i32 + 1;
                    match card.value as i32 {
                        0 => {
                            if !dealer_total.1 .0 {
                                dealer_total.1 .1 += card.value as i32 + 11;
                                dealer_total.1 .0 = true;
                            } else {
                                dealer_total.1 .1 += card.value as i32 + 1;
                            }
                        }
                        _ => dealer_total.1 .1 += card.value as i32 + 1,
                    }
                }
                if (dealer_total.0 > player_total.0 || dealer_total.0 > player_total.1 .1)
                    || (dealer_total.1 .1 > player_total.0 || dealer_total.1 .1 > player_total.1 .1)
                        && dealer_total.0 <= 21
                {
                    true
                } else {
                    false
                }
            }
            None => false,
        }
    }

    pub fn check_winners_player(&self) -> Vec<Uuid> {
        let mut winners: Vec<Uuid> = Vec::new();
        for player in self.players.iter() {
            if self.check_winner(player.id) {
                winners.push(player.id);
            }
        }
        winners
    }
}
