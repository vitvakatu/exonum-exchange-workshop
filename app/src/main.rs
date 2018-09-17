extern crate web_logger;
extern crate yew;
extern crate trading;

use yew::prelude::*;
use yew::services::fetch::FetchService;
use trading::context::Context;
use trading::Model;

fn main() {
    web_logger::init();
    yew::initialize();
    let context = Context::new();
    let app: App<_, Model> = App::new(context);
    app.mount_to_body();
    yew::run_loop();
}
