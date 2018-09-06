extern crate coinbase_pro_rs;
extern crate orderbook_rs;
extern crate futures;
extern crate tokio;
extern crate tokio_tungstenite;

use futures::{Future, Stream};
use futures::future::Either;
use coinbase_pro_rs::{WSFeed, WS_URL};
use coinbase_pro_rs::structs::wsfeed::*;
use coinbase_pro_rs::{Public, ASync, MAIN_URL, WSError};
use coinbase_pro_rs::structs::public::*;
use orderbook_rs::{OrderBook, BookRecord};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use coinbase_pro_rs::structs::reqs::OrderSide;
use orderbook_rs::Side;

fn convert_side(side: OrderSide) -> Side {
    match side {
        OrderSide::Buy => Side::Buy,
        OrderSide::Sell => Side::Sell
    }
}

fn convert_record(rec: &BookRecordL3) -> BookRecord {
    BookRecord{price: rec.price, size: rec.size, id: rec.order_id}
}

fn get_seq(full: &Full) -> &usize {
    println!("{:?}", full);
    match full {
        Full::Open(Open{sequence, ..}) => sequence,
        Full::Done(Done::Limit{sequence, ..}) => sequence,
        Full::Done(Done::Market{sequence, ..}) => sequence,
        Full::Match(Match{sequence, ..}) => sequence,
        Full::Change(Change{sequence, ..}) => sequence,
        Full::Received(Received::Limit{sequence, ..}) => sequence,
        Full::Received(Received::Market{sequence, ..}) => sequence,
        _ => unimplemented!()
    }
}

fn reload(client: &Public<ASync>) -> impl Future<Item=Book<BookRecordL3>, Error=WSError>{
    println!("reload");
    client.get_book("BTC-USD")
        .map_err(|_| WSError::Read(tokio_tungstenite::tungstenite::Error::Http(10)))
}

fn process_full(ob: &mut OrderBook, full: Full) {
    match full {
        Full::Open(Open{price, remaining_size: size, order_id: id, side, ..})
        => ob.open(convert_side(side), BookRecord { price, size, id }).unwrap_or(()),
        Full::Done(Done::Limit {price, order_id: id, ..})
        => ob.done(price, id).unwrap_or(()),
        Full::Match(Match{size, price, maker_order_id: id, ..})
        => ob._match(price, size, id).unwrap_or(()),
        Full::Change(Change{new_size: size, price, order_id: id, ..})
        => ob.change(price, size, id).unwrap_or(()),
        _ => println!("other")
    }
}

fn main() {
    let sequence = Arc::new(AtomicUsize::new(0));

    let client: Public<ASync> = Public::new(MAIN_URL);
    let ob = Arc::new(Mutex::new(OrderBook::new()));

    let stream = WSFeed::new(WS_URL, &["BTC-USD"], &[ChannelType::Full]);

    let f = stream
        .for_each(move |msg| {
            match msg {
               Message::Full(full) => {
                   let new_sequence = get_seq(&full).to_owned();
                   let old_sequence = sequence.load(Ordering::SeqCst);
                   println!("msg: seq: {}    old: {}", new_sequence, old_sequence);
                   if new_sequence > 1 + old_sequence {
                       let ob2 = ob.clone();
                       let sequence2 = sequence.clone();
                       return Either::A(reload(&client)
                           .and_then(move |book| {
                               println!("reload: done: seq: {}", book.sequence);
                               let bids = book.bids.iter()
                                   .map(convert_record)
                                   .collect::<Vec<_>>();
                               let asks = book.bids.iter()
                                   .map(convert_record)
                                   .collect::<Vec<_>>();
                               let mut ob = ob2.lock().unwrap();
                               ob.init(bids, asks);
                               {
                                   sequence2.store(book.sequence, Ordering::SeqCst);
                               }
                               Ok(())
                           }))
                   } else if new_sequence <= old_sequence {
                       ;
                   } else {
                       sequence.fetch_add(1, Ordering::SeqCst);
                       let mut ob = ob.lock().unwrap();
                       process_full(&mut ob, full);
                       println!("{}", ob);
                   }
                   },
               _ => ()
            }
            Either::B(futures::future::result(Ok(())))
        });

    tokio::run(f.map_err(|e| println!("{:?}", e)));
}

