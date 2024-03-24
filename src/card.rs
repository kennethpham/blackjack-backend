use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter};

#[derive(Clone, Copy, Debug, Deserialize, Display, EnumIter, Serialize)]
pub enum CardValue {
    Ace,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
}
#[derive(Clone, Copy, Debug, Deserialize, Display, EnumIter, Serialize)]
pub enum CardSuit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Card {
    pub value: CardValue,
    pub suit: CardSuit,
}

pub fn get_card_file(value: String, suit: String) -> Result<String, &'static str> {
    match value.to_lowercase().as_str() {
        "ace" => CardValue::Ace,
        "2" => CardValue::Two,
        "3" => CardValue::Three,
        "4" => CardValue::Four,
        "5" => CardValue::Five,
        "6" => CardValue::Six,
        "7" => CardValue::Seven,
        "8" => CardValue::Eight,
        "9" => CardValue::Nine,
        "10" => CardValue::Ten,
        "jack" => CardValue::Jack,
        "queen" => CardValue::Queen,
        "king" => CardValue::King,
        _ => return Err("invalid card value"),
    };

    let card_suit: CardSuit = match suit.to_lowercase().as_str() {
        "clubs" => CardSuit::Clubs,
        "diamonds" => CardSuit::Diamonds,
        "hearts" => CardSuit::Hearts,
        "spades" => CardSuit::Spades,
        _ => return Err("invalid card suit"),
    };

    Ok(format!(
        "{}_of_{}",
        value.to_string().to_lowercase(),
        card_suit.to_string().to_lowercase(),
    ))
}
