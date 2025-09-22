use std::{sync::atomic::AtomicU8, thread::sleep, time::Duration};

use tracing::Level;

use crate::{handle::Planetary, task::Runnable};

fn enable_tracing() {
    drop(tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .try_init());
}

fn thread_name() -> String {
    std::thread::current().name().unwrap_or("Unnamed-thread").to_string()
}

struct SleepFor {
    duration: Duration
}

impl Runnable for SleepFor {
    type Output = u64;

    fn run(self) -> Self::Output {
        println!("[{}] Sleeping for {:?}", thread_name(), self.duration);
        sleep(self.duration);
        tracing::info!("[{}] Slept for {:?}", thread_name(), self.duration);
        self.duration.as_secs()
    }
}

fn create_pool(threads: usize, launch: bool) -> Planetary {
    Planetary::builder()
        .max_threads(threads)
        .launch_on_build(launch)
        .with_hooks(|hooks| {
            hooks.set_on_start_fn(|| {
                println!("Thread {} started", thread_name());
            }).set_on_stop_fn(|| {
                println!("Thread {} stopped", thread_name());
            }).set_on_park_fn(|| {
                println!("Thread {} parked", thread_name());
            }).set_on_unpark_fn(|| {
                println!("Thread {} unparked", thread_name());
            }).set_before_work_fn(|| {
                println!("Thread {} started work item", thread_name());
            }).set_after_work_fn(|| {
                println!("Thread {} finished work item", thread_name());
            }).set_name_fn(|| {
                static THREAD_ID: AtomicU8 = AtomicU8::new(0);

                format!("Worker-{}", THREAD_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
            });
        })
        .build()
        .unwrap()
}

#[test]
fn create_shutdown() {
    enable_tracing();
    create_pool(2, false).shutdown();
}

#[test]
fn create_launch_shutdown() {
    enable_tracing();
    let handle = create_pool(2, true);
    sleep(Duration::from_secs(2));
    handle.shutdown();
}

#[test]
fn create_spawn_shutdown() {
    enable_tracing();
    let handle = create_pool(2, false);
    handle.spawn(SleepFor {
        duration: Duration::from_secs(5)
    });
    handle.spawn(SleepFor {
        duration: Duration::from_secs(2)
    });

    handle.shutdown();
}

#[test]
fn create_spawn_take_from_injector() {
    enable_tracing();

    let handle = create_pool(1, false);

    handle.spawn(SleepFor {
        duration: Duration::from_secs(5)
    });

    handle.spawn(SleepFor {
        duration: Duration::from_secs(8)
    }); // wont execute, just to confirm goes to the injector

    handle.shutdown();
}

#[test]
fn spawn_inside_worker_single_worker() {
    enable_tracing();
    let handle = create_pool(1, false);

    handle.spawn(|| {
        println!("Spawning task into current worker");
        crate::spawn(|| {
            println!("Hello from nested task!");
        });
        12
    });

    sleep(Duration::from_secs(2));

    handle.shutdown();
}

#[test]
fn steal_work() {
    enable_tracing();

    let handle = create_pool(2, false);

    handle.spawn(SleepFor {
        duration: Duration::from_secs(4)
    });

    handle.spawn(|| {
        crate::spawn(SleepFor {
            duration: Duration::from_secs(5)
        });
        println!("[{}] Sleeping 10 secs", thread_name());
        sleep(Duration::from_secs(10));
        println!("[{}] Slept 10 secs", thread_name());
    });

    sleep(Duration::from_secs(15));
    println!("Shutdown");
    handle.shutdown();
}
