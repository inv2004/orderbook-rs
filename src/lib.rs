extern crate uuid;

use std::ops::RangeInclusive;
use std::fmt;
use std::collections::VecDeque;
pub use uuid::Uuid;

pub enum Side {
    Buy, Sell
}

pub struct BookRecord {
    pub price: f64,
    pub size: f64,
    pub id: Uuid
}

pub struct OrderBook {
    pub book: Vec<VecDeque<(f64, Uuid)>>,
    bid: usize,
    ask: usize
}

fn get_idx(price: f64) -> usize {
    (price * 100.0) as usize
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            book: vec![VecDeque::new(); 20000*100],
            bid: std::usize::MIN,
            ask: std::usize::MAX
        }
    }

    pub fn init(&mut self, bids: Vec<BookRecord>, asks: Vec<BookRecord>) {
        bids.into_iter().for_each(|rec| self.add(Side::Buy, rec));
        asks.into_iter().for_each(|rec| self.add(Side::Sell, rec));
    }

    pub fn add(&mut self, side: Side, rec: BookRecord) {
        let p_idx = get_idx(rec.price);
        match side {
            Side::Buy if p_idx > self.bid => self.bid = p_idx,
            Side::Sell if p_idx < self.ask => self.ask = p_idx,
            _ => ()
        }
        self.book[p_idx].push_back((rec.size, rec.id));
    }

    pub fn _match(&mut self, price: f64, size: f64, id: Uuid) {
        let p_idx = get_idx(price);
        assert_eq!(id, self.book[p_idx][0].1);
        let mut sz = self.book[p_idx][0].0;
        sz -= size;
        if sz == 0.0 {
            self.book[p_idx].pop_front();
            self.check_ask_bid(p_idx);
        }
    }

    pub fn done(&mut self, price: f64, id: Uuid) {
        let p_idx = get_idx(price);
        self.book[p_idx].retain(|&(_, it_id)| it_id != id);
        self.check_ask_bid(p_idx);
    }

    pub fn change(&mut self, price: f64, new_size: f64, id: Uuid) {
        let p_idx = get_idx(price);
        if new_size == 0.0 {
            self.done(price, id);
        } else {
            self.book[p_idx]
                .iter_mut()
                .for_each(|(it_size, it_id)| {
                    if *it_id == id {
                        *it_size = new_size;
                    }
                })
        }
    }

    fn check_ask_bid(&mut self, p_idx: usize) {
        if p_idx == self.bid {
            while self.book[self.bid].len() == 0 {
                self.bid -= 1;
            }
        }

        if p_idx == self.ask {
            while self.book[self.ask].len() == 0 {
                self.ask += 1;
            }
        }
    }
}

impl OrderBook {
    fn fmt_side(&self, range: RangeInclusive<usize>) -> String {
        self.book[range]
            .into_iter()
            .map(|orders| {
                let size = orders.iter().map(|x| x.0).sum::<f64>();
                let _count = orders.len();
                size.to_string()
            })
            .collect::<Vec<_>>()
            .join(",")
    }
}

impl fmt::Display for OrderBook {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let size = 20;
        let bids = self.fmt_side((self.bid-size)..=self.bid);
        let asks = self.fmt_side(self.ask..=self.ask+size);
        let bid = self.bid as f64 / 100.0;
        let ask = self.ask as f64 / 100.0;
        write!(f, "OB: {} | {:.2}   {:.2} | {}", bids, bid, ask, asks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let mut ob = OrderBook::new();
        ob.init(vec![BookRecord{price:3994.96, size:0.3, id:Uuid::new_v4()}
                     , BookRecord{price:3995.0, size:0.5, id:Uuid::new_v4()}],
                vec![BookRecord{price:4005.0, size:0.4, id:Uuid::new_v4()}
                     , BookRecord{price: 4005.02, size: 0.2, id: Uuid::new_v4()}]);

        ob.add(Side::Buy, BookRecord{price:3994.96, size:0.2, id: Uuid::new_v4()});

        let str = format!("{}", ob);
        assert_eq!(str, "OB: 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.5,0,0,0,0.5 | 3995.00   4005.00 | 0.4,0,0.2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0");
    }

    #[test]
    fn test_match() {
        let mut ob = OrderBook::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        ob.init(vec![BookRecord{price:3994.96, size:0.3, id:id1}
                     , BookRecord{price:3995.0, size:0.5, id:id2}],
                vec![BookRecord{price:4005.0, size:0.4, id:Uuid::new_v4()}
                     , BookRecord{price: 4005.02, size: 0.2, id: Uuid::new_v4()}]);

        ob.add(Side::Buy, BookRecord{price:3994.96, size:0.2, id:Uuid::new_v4()});
        ob._match(3995.0, 0.5, id2);

        let str = format!("{}", ob);
        assert_eq!(str, "OB: 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.5 | 3994.96   4005.00 | 0.4,0,0.2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0");
    }

    #[test]
    fn test_done() {
        let mut ob = OrderBook::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        ob.init(vec![BookRecord{price:3994.96, size:0.3, id:id1}
                     , BookRecord{price:3995.0, size:0.5, id:id2}],
                vec![BookRecord{price:4005.0, size:0.4, id:Uuid::new_v4()}
                     , BookRecord{price: 4005.02, size: 0.2, id: Uuid::new_v4()}]);

        ob.done(3994.96, id1);

        let str = format!("{}", ob);
        assert_eq!(str, "OB: 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.5 | 3995.00   4005.00 | 0.4,0,0.2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0");
    }
}
