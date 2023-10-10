mod gui;
pub mod backend;

//use std::env;

use gui::GitUI;

pub fn main() {
    //env::set_current_dir("/home/main/testrepo").unwrap();
    GitUI::start()
}
