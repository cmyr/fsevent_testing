extern crate tempdir;
extern crate fsevent;
extern crate test_helpers;
extern crate xattr;
extern crate chrono;

use std::sync::mpsc::channel;
use std::thread;
use std::time::{Instant, Duration};
use std::fs::{File, OpenOptions};
use std::io::Write;

use test_helpers::PrettyDuration;
use tempdir::TempDir;
use chrono::prelude::*;


fn main() {
    for i in 0..10 {
        println!("run {}", i + 1);
        run_tests()
    }
}

fn run_tests() {
    let (sender, receiver) = channel();

    let dir = TempDir::new("fsevent_test")
        .expect("create tmp dir");
    let dir_str = dir.path().to_str().unwrap().to_owned();

    let _t = thread::spawn(move || {
        let fsevent = fsevent::FsEvent::new(sender);
        fsevent.append_path(&dir_str);
        fsevent.observe();
    });

    let mut ticks = 0;
    let file_path = dir.path().join(format!("test.file{}", ticks));
    let start = Instant::now();

    {
        File::create(&file_path).expect("create failed");
    }

    receiver.recv_timeout(Duration::from_secs(1)).expect("no initial event");
    //println!("Testing 'create' flag stickiness");
    loop {
        thread::sleep(Duration::from_millis(1000));
        ticks += 1;
        {
            let mut f = OpenOptions::new().append(true).open(&file_path).unwrap();
            writeln!(f, "this is line {}", ticks).expect("write failed");
        }
        let e = receiver.recv_timeout(Duration::from_secs(1))
            .expect(&format!("tick {} timeout", ticks));
        if !e.flag.contains(fsevent::ITEM_CREATED) {
            let now = Local::now();
            println!("Create flag cleared {}, t: {:?}",
                     PrettyDuration::new(start.elapsed()),
                     now.time());
            break
        }
        if start.elapsed() > Duration::from_secs(60) {
            println!("Create test timeout");
            break
        }
    }

    ticks = 0;
    let start = Instant::now();
    //println!("Testing 'modify' flag stickiness");
    loop {
        thread::sleep(Duration::from_millis(1000));
        ticks += 1;
        xattr::set(&file_path, format!("xattr-{}", ticks), "test.val".as_bytes())
            .expect("xattr set failed");
        let e = receiver.recv_timeout(Duration::from_secs(1))
            .expect(&format!("tick {} timeout", ticks));
        if !e.flag.contains(fsevent::ITEM_MODIFIED) {
            println!("Modify flag cleared {}", PrettyDuration::new(start.elapsed()));
            break
        }
        if start.elapsed() > Duration::from_secs(60) {
            println!("Modify test timeout");
            break
        }
    }
}

