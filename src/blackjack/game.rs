use mongodb::bson::uuid::Uuid;

struct PlayerCard {
    suit: String,
    value: String,
    visible: bool,
}

struct Player {
    id: Uuid,
    hand: Vec<PlayerCard>,
}

struct Table {
    id: Uuid,
    players: Vec<Player>,
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
        });
        self.tables.last().unwrap().id
    }
}

impl Table {
    pub fn create_table() -> Table {
        let id = Uuid::new();
        let players: Vec<Player> = Vec::new();
        Table { id, players }
    }

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
}

impl Player {
    pub fn get_cards(&self) -> &Vec<PlayerCard> {
        self.hand.as_ref()
    }

    pub fn add_card(&mut self) {
        // TODO: ADD CARD LOGIC
    }

}
