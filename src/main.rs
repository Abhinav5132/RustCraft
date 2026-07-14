use crate::app::App;
pub mod app;
pub mod camera;
pub mod inputs;
pub mod texture;
pub mod vertex;
pub mod window;

pub fn main() {
    App::run().unwrap();
}
