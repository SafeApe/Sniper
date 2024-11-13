#!/usr/bin/env rust-script
//! # sub.rs
//! ```cargo
//! [dependencies]
//! iceoryx2 = "0.4.1"
//! ```

use core::time::Duration;
use iceoryx2::prelude::{ipc, NodeBuilder, NodeEvent};

const CYCLE_TIME: Duration = Duration::from_secs(1);

pub fn main() {
    let node = NodeBuilder::new().create::<ipc::Service>().unwrap();
    let event = node
        .service_builder(&"MyEventName".try_into().unwrap())
        .event()
        .open_or_create()
        .unwrap();

    let mut listener = event.listener_builder().create().unwrap();

    while let NodeEvent::Tick = node.wait(Duration::ZERO) {
        if let Ok(Some(event_id)) = listener.timed_wait_one(CYCLE_TIME) {
            println!("event was triggered with id: {:?}", event_id);
        }
    }
}
