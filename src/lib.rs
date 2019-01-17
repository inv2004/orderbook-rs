//! # orderbook-rs
//!
//! I did this orderbook in addition to <https://github.com/inv2004/coinbase-pro-rs>
//!
//! For performance I put it in flat array, that is why it uses memory actively.
//! For current coinbase BTC-USD pair it takes 188.1 Mb or RAM.
//!
//! It has hardcoded limit for 20000(max price) * 100(cents) = 2*10^6 values it can store.
//! Call the OB with values which are outside these boundaries will return None,
//! but, I suppose, this return can be ignored in most cases.
//!

extern crate uuid;
#[macro_use] extern crate failure;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "range")]
    Range,
    #[fail(display = "bid_less_ask")]
    BidLessAsk,
    #[fail(display = "match_uuid")]
    MatchUuid,
    #[fail(display = "test price is not bid or ask")]
    TestFail
}

pub(crate) const MAX_SIZE: usize = 20000 * 100;

#[derive(Debug)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug)]
pub struct BookRecord {
    pub price: f64,
    pub size: f64,
    pub id: uuid::Uuid,
}

pub mod ob;
pub use ob::OrderBook;
