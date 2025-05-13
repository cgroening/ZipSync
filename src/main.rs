mod controller;
mod model;
use crate::controller::main_controller::MainController;


fn main() {
    // Start main controller
    MainController::new().start();
}
