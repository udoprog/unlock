use std::sync::Arc;
use std::thread;

use unlock::RwLock;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let lock = Arc::new(RwLock::new(0u64));

    let mut threads = Vec::new();

    unlock::capture();

    for _ in 0..100 {
        let lock = lock.clone();

        threads.push(thread::spawn(move || {
            let mut sum = 0u64;

            for n in 0..100 {
                if n % 4 == 0 {
                    *lock.write() += 1;
                } else {
                    sum += *lock.read();
                }

                std::thread::sleep(std::time::Duration::from_millis(n % 11));
            }

            sum
        }));
    }

    let mut total = 0;

    for thread in threads {
        total += thread.join().unwrap();
    }

    dbg!(total);

    let events = unlock::drain();
    dbg!(events.len());
    unlock::html::write("trace.html", &events)?;
    Ok(())
}
