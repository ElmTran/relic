// Release 下不弹控制台窗口，不要删
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    relic_lib::run()
}
