extern crate coinbase_pro_rs;
extern crate futures;
extern crate orderbook;
extern crate tokio;
extern crate tokio_tungstenite;
#[macro_use]
extern crate failure;

use coinbase_pro_rs::structs::public::*;
use coinbase_pro_rs::structs::wsfeed::*;
use coinbase_pro_rs::{ASync, Public, MAIN_URL};
use coinbase_pro_rs::{WSFeed, WS_URL};
use futures::future::Either;
use futures::{Future, Stream};
use orderbook::{BookRecord, OrderBook};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use coinbase_pro_rs::structs::reqs::OrderSide;
use orderbook::Side;

#[derive(Debug, Fail)]
enum Error {
    #[fail(display = "coinbase")]
    Coinbase(#[cause] coinbase_pro_rs::CBError),
    #[fail(display = "websocket")]
    Websocket(#[cause] coinbase_pro_rs::WSError),
}

fn convert_side(side: OrderSide) -> Side {
    match side {
        OrderSide::Buy => Side::Buy,
        OrderSide::Sell => Side::Sell,
    }
}

fn convert_record(rec: &BookRecordL3) -> BookRecord {
    BookRecord {
        price: rec.price,
        size: rec.size,
        id: rec.order_id,
    }
}

fn get_seq(full: &Full) -> Option<&usize> {
    match full {
        Full::Received(Received::Limit { sequence, .. }) => Some(sequence),
        Full::Received(Received::Market { sequence, .. }) => Some(sequence),
        Full::Open(Open { sequence, .. }) => Some(sequence),
        Full::Done(Done::Limit { sequence, .. }) => sequence.as_ref(),
        Full::Done(Done::Market { sequence, .. }) => Some(sequence),
        Full::Match(Match { sequence, .. }) => Some(sequence),
        Full::Change(Change { sequence, .. }) => Some(sequence),
        _ => None,
    }
}

fn reload(client: &Public<ASync>) -> impl Future<Item = Book<BookRecordL3>, Error = Error> {
    println!("reload");
    client.get_book("BTC-USD").map_err(Error::Coinbase)
}

fn process_full(ob: &mut OrderBook, full: Full) {
    //    println!("{:?}", full);
    match full {
        Full::Received(..) => (),
        Full::Open(Open {
            price,
            remaining_size: size,
            order_id: id,
            side,
            ..
        }) => ob
            .open(convert_side(side), BookRecord { price, size, id })
            .unwrap_or(()),
        Full::Done(Done::Limit {
            price,
            order_id: id,
            ..
        }) => ob.done(price, id).unwrap_or(()),
        Full::Done(Done::Market { .. }) => (),
        Full::Match(Match {
            size,
            price,
            maker_order_id: id,
            ..
        }) => ob._match(price, size, id).unwrap_or(()),
        Full::Change(Change {
            new_size: size,
            price: Some(price),
            order_id: id,
            ..
        }) => ob.change(price, size, id).unwrap_or(()),
        _ => println!("other: {:?}", full),
    }
}

fn main() {
    let sequence = Arc::new(AtomicUsize::new(0));

    let client: Public<ASync> = Public::new(MAIN_URL);
    let ob = Arc::new(Mutex::new(OrderBook::new()));

    let stream = WSFeed::new(WS_URL, &["BTC-USD"], &[ChannelType::Full]);

    let f = stream
        .map_err(Error::Websocket)
        .for_each(move |msg| {
        match msg {
            Message::Full(full) => {
                let new_sequence = get_seq(&full).unwrap().to_owned();
                let old_sequence = sequence.load(Ordering::SeqCst);
                if new_sequence > 1 + old_sequence {
                    let ob2 = ob.clone();
                    let sequence2 = sequence.clone();
                    return Either::A(reload(&client).and_then(move |book| {
                        let bids = book.bids.iter().map(convert_record).collect::<Vec<_>>();
                        let asks = book.asks.iter().map(convert_record).collect::<Vec<_>>();
                        let mut ob = ob2.lock().unwrap();
                        ob.reload(bids, asks).unwrap_or_default();
                        sequence2.store(book.sequence, Ordering::SeqCst);
                        println!("{}", ob);
                        Ok(())
                    }));
                } else if new_sequence <= old_sequence {
                    ()
                } else {
                    sequence.fetch_add(1, Ordering::SeqCst);
                    let mut ob = ob.lock().unwrap();
                    process_full(&mut ob, full);
                    println!("{}", ob);
                }
            }
            _ => ()
        }
        Either::B(futures::future::result(Ok(())))
    });

    tokio::run(f.map_err(|e| println!("{:?}", e)));
}
