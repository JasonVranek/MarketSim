use tokio::net::tcp::TcpStream;
use crate::order::order::{Order, OrderType, TradeType, ExchangeType};
use crate::blockchain::mem_pool::MemPool;

use crate::log_mempool_data;

use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

extern crate serde;
extern crate serde_json;
extern crate tokio_serde_json;

use tokio::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use serde_json::Value;
use tokio_serde_json::{ReadJson, WriteJson};
use log::{log, Level};

// Handles JSON serialization/deserialization functions and new message processing
pub struct OrderProcessor {}


impl OrderProcessor {
	// Preprocess message in a new thread and append to MemPool
	// order is the trader's order that this function takes ownership of
	// pool is an Arc clone of the MemPool stored on the heap
	pub fn conc_recv_order(order: Order, pool: Arc<MemPool>) -> JoinHandle<()> {
	    thread::spawn(move || {
	    	// Log the order to the mempool logger
	    	log_mempool_data!(Order::order_to_csv(&order));
	    	// The add function acquires the lock
	    	pool.add(order);
	    })
	}
}

// Type alias for returning JSON stream
type DeserializedStream = ReadJson<FramedRead<TcpStream, LengthDelimitedCodec>, serde_json::Value>;
type SerializedStream = WriteJson<FramedWrite<TcpStream, LengthDelimitedCodec>, serde_json::Value>;

// A struct for providing stong types to deserialize the incoming JSONs
#[derive(Deserialize, Debug)]
pub struct JsonOrder{
	trader_id: String,
	order_type: String,	
	trade_type: String,  
	ex_type: String,
	p_low: f64,				
	p_high: f64,
	price: f64,
	quantity: f64,		
	u_max: f64,
	gas: f64,	
}

impl JsonOrder {
	pub fn serializer(socket: TcpStream) -> SerializedStream{
		// Delimit frames using a length header
	    let length_delimited = FramedWrite::new(socket, LengthDelimitedCodec::new());

	    // Serialize frames
	    let serializer = WriteJson::new(length_delimited);

	    serializer
	}

	pub fn deserialize(socket: TcpStream) ->  DeserializedStream {
		// Delimit frames using a length header
	    let length_delimited = FramedRead::new(socket, LengthDelimitedCodec::new());

	    // Deserialize frames
	    let deserialized = ReadJson::<_, Value>::new(length_delimited);

	    deserialized
	}
	// Deserialize the JSON, create an Order type, and push onto the queue
	pub fn process_new(msg: serde_json::Value, queue: Arc<MemPool>) {
		// create Order from JSON
		let order = JsonOrder::order_from_json(msg);

		if let Some(o) = order {
			// add message to queue with conc_recv_order()
			let handle = OrderProcessor::conc_recv_order(o, Arc::clone(&queue));
			handle.join().unwrap();
		} else {
			println!("Unsuccessful json parsing");
		}
	}

	// Make an Order from a JSON
	fn order_from_json(msg: serde_json::Value) -> Option<Order> {
		let typed_json: JsonOrder = serde_json::from_value(msg).expect("Couldn't make JSON");
		// Parse JSON body into enums compatible with flow market
		let ot = match typed_json.order_type.to_lowercase().as_ref() {
			"enter" => OrderType::Enter,
			"update" => OrderType::Update,
			"cancel" => OrderType::Cancel,
			_ => {
				println!("Entered an invalid ordertype!");
				return None;
				},
		};

		let tt = match typed_json.trade_type.to_lowercase().as_ref() {
			"bid" => TradeType::Bid,
			"ask" => TradeType::Ask,
			_ => {
				println!("Entered an invalid tradetype");
				return None;
			},
		};

		let et = match typed_json.ex_type.to_lowercase().as_ref() {
			"floworder" => ExchangeType::FlowOrder,
			"limitorder" => ExchangeType::LimitOrder,
			_ => {
				println!("Entered an invalid tradetype");
				return None;
			},
		};

		// let func = match tt {
		// 	TradeType::Bid => p_wise_dem(typed_json.p_low, typed_json.p_high, typed_json.u_max),
		// 	TradeType::Ask => p_wise_sup(typed_json.p_low, typed_json.p_high, typed_json.u_max),
		// };

		Some(Order::new(
			typed_json.trader_id,
			ot, 
			tt, 
			et,
			typed_json.p_low,
			typed_json.p_high,
			typed_json.price, 
			typed_json.quantity, 
			typed_json.u_max,
			typed_json.gas,
			))
	}

	// Turn an order into JSON from its params
	pub fn order_to_json(order: &Order) -> serde_json::Value {
		let ot = match order.order_type {
            OrderType::Enter => "enter",
            OrderType::Update => "update",
            OrderType::Cancel => "cancel",
        };

        let tt = match order.trade_type {
            TradeType::Bid => "bid",
            TradeType::Ask => "ask",
        };

		json!({
                "trader_id": order.trader_id.clone(),
                "order_type": ot,
                "trade_type": tt,
                "price": order.price.clone(),
                "quantity": order.quantity.clone(),
            })
	}

	pub fn params_to_json(order_params: (String, OrderType, TradeType, f64, f64)) 
	-> serde_json::Value {
		let (t_id, ot, tt, p, q) = order_params;

		let ot = match ot {
            OrderType::Enter => "enter",
            OrderType::Update => "update",
            OrderType::Cancel => "cancel",
        };

        let tt = match tt {
            TradeType::Bid => "bid",
            TradeType::Ask => "ask",
        };

		json!({
                "trader_id": t_id,
                "order_type": ot,
                "trade_type": tt,
                "price": p,
                "quantity": q,
            })
	}
}




