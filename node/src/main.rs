#[macro_use]
extern crate log;
extern crate cryptoexchange;
extern crate exonum;

use cryptoexchange::service;
use exonum::helpers::fabric::NodeBuilder;

fn main() {
    exonum::helpers::init_logger().unwrap();
    NodeBuilder::new()
        .with_service(Box::new(service::ExchangeService))
        .run();
}
