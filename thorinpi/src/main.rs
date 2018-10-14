extern crate env_logger;
extern crate gpio_sensors;
extern crate libc;
#[macro_use]
extern crate log;
extern crate reqwest;
extern crate rppal;
#[macro_use]
extern crate serde_derive;
extern crate toml;

mod core;
mod rig;
mod vent;
mod sensor;

use core::Settings;
use rig::{Rig, RigCheckResult};
use sensor::TSensor;
use vent::Vent;

use std::thread;
use std::time::Duration;

use std::env;
use std::fs::File;
use std::io::Error;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::exit;
use std::rc::Rc;
use std::cell::RefCell;
use std::error::Error as ErrorStd;

pub fn read_file(p: &PathBuf) -> Result<String, Error> {
    File::open(p).and_then(|mut f| {
        let mut s = String::new();
        f.read_to_string(&mut s).map(|_| s)
    })
}

fn print_usage(p: &str) {
    println!("\nUsage: {} ./path/to/config.toml", p);
}

fn build_logger() -> env_logger::Builder {
    let mut b = env_logger::Builder::new();
    b.target(env_logger::Target::Stderr);
    b.format(|buf, record| writeln!(buf, "{}: {}", record.level(), record.args()));
    if env::var("RUST_LOG").is_ok() {
        b.parse(&env::var("RUST_LOG").unwrap());
    }
    b
}

fn main() {
    let mut log = build_logger();
    log.init();
    // env_logger::init();
    info!("Logger initialized!");
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let toml_path = if args.len() > 1 {
        Path::new(&args[1])
    } else {
        print_usage(&program);
        exit(1);
    };

    // if !toml_path.exists() || !toml_path.is_file() {
    //     print_usage(&program);
    //     exit(1);
    // }

    let settings = read_file(&toml_path.to_path_buf())
        .map(|it| {
            toml::from_str::<Settings>(&it).expect(&format!(
                "Error parsing TOML file {}",
                toml_path.to_string_lossy()
            ))
        })
        .expect(&format!(
            "Error reading TOML file {}",
            toml_path.to_string_lossy()
        ));

    // match DeviceInfo::new() {
    //     Ok(di) => {
    //         println!("Model: {} (SoC: {})", di.model(), di.soc());
    //     }
    //     Err(e) => println!("ERROR {}", e),
    // }

    info!("SETTINGS LOADED\n{:?}", settings);
    application(settings);
    exit(0);
}

fn application(settings: Settings) {
    let mut rigs: Vec<Rig> = Vec::new();
    let mut sensors = Vec::<Rc<RefCell<TSensor>>>::new();
    let mut vents = Vec::<Vent>::new();

    for rig in &settings.rigs {
        rigs.push(Rig::new(rig));
    }

    for s in &settings.sensors {
        sensors.push(Rc::new(RefCell::new(TSensor::new(s))));
    }

    debug!("Sensors loaded {:?}", sensors);

    for v in &settings.vents {
        vents.push(Vent::new(v, &sensors));
    }
    debug!("Sensors after vents {:?}", sensors);

    let mut cycle = 0;

    loop {
        // let ref mut ss:Vec<Rc<TSensor>> = sensors;
        let mut gpu_temps = Vec::<isize>::new();
        for r in &mut rigs {
            if let Some(mut res) = r.handle() {
                gpu_temps.append(&mut res.temp.clone());
                if cycle % 60 == 0 {
                    show_rig_check(&res);
                }
            }
            // println!("RESULT {:?}", h);
        }

        if cycle % 60 == 0 {
            info!("Temperatures: {}", format_temperature(&sensors));
        }

        // Ventilation stuff
        for v in &mut vents {
            v.handle(&gpu_temps);
        }

        cycle = (cycle + 1) % 1000_000;
        thread::sleep(Duration::from_millis(1000));
    }
}

fn format_temperature(sensors: &Vec<Rc<RefCell<TSensor>>>) -> String {
    sensors
        .into_iter()
        .map(|s| s.clone())
        .map(|rc| {
            rc.try_borrow_mut()
                //.ok_or(String::from("Can not get mutable access"))
                .map_err(|e| String::from(e.description()))
                .and_then(|ref mut s| s.temperature())
        })
        .filter_map(|s| s.ok())
        .fold(String::new(), |acc, num| acc + &num.to_string() + "C ")
}

fn show_rig_check(check: &RigCheckResult) {
    info!(
        "{} led_on:{} service:{} errors:{} temps:{:?}",
        check.hostname,
        check.led_on.unwrap_or(false),
        check.service,
        check.hw_errors,
        check.temp.clone() // check
                           //     .temp
                           //     .clone()
                           //     .into_iter()
                           //     .fold(String::new(), |acc, num| acc + &num.to_string() + ", ")
    );
}
