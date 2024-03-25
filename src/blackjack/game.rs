use mongodb::bson::uuid::Uuid;
use strum::IntoEnumIterator;

use crate::card::{Card, CardSuit, CardValue};

use rand::{seq::SliceRandom, thread_rng};

// Number of decks used by table.
const NUM_OF_DECKS: u8 = 3;

struct PlayerCard {
    pub card: Card,
    visible: bool,
}

struct Player {
    id: Uuid,
    hand: Vec<PlayerCard>,
}

struct Table {
    id: Uuid,
    players: Vec<Player>,
    deck: Vec<Card>,
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

    pub fn get_player(&mut self, id: Uuid) -> Option<usize> {
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

    // TODO: pub fn check_winners(self) -> Vec<Player> {}
}

impl Player {
    pub fn get_cards(&self) -> &Vec<PlayerCard> {
        self.hand.as_ref()
    }

    pub fn add_card(&mut self) {
        // TODO: ADD CARD LOGIC
    }
}
