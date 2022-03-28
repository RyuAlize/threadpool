mod task;
mod pool;

use pool::ThreadPool;
use task::Task;
use std::thread::sleep;
use std::time;

struct Wo {
    id: usize,
}
impl Task for Wo {
    fn run(self) {
        let ten_millis = time::Duration::from_millis(1000);
        println!("hello, {}", self.id);

        sleep(ten_millis);
    }
}
fn main() {
    let (sender, pool ) = pool::ThreadPool::new(10, 100);
    for i in 0..400 {

        sender.add_task(wo{id:i});
    }
    let ten_millis = time::Duration::from_millis(10000);
    sleep(ten_millis);
}
