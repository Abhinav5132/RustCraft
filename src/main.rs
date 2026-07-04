use crate::app::App;
use anyhow::Result;
pub mod app;
pub mod window;

pub fn main() {
    App::run().unwrap();
}
