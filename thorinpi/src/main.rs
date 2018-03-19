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
use rig::Rig;
use sensor::TSensor;
use vent::Vent;

use std::thread;
use std::time::{Duration};

use std::env;
use std::fs::File;
use std::io::Error;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::exit;
use std::rc::Rc;

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
            "Error readin TOML file {}",
            toml_path.to_string_lossy()
        ));

    // match DeviceInfo::new() {
    //     Ok(di) => {
    //         println!("Model: {} (SoC: {})", di.model(), di.soc());
    //     }
    //     Err(e) => println!("ERROR {}", e),
    // }

    trace!("SETTINGS {:?}", settings);
    application(settings);
    exit(0);
}

fn application(settings: Settings) {
    let mut rigs: Vec<Rig> = Vec::new();
    let mut sensors = Vec::<Rc<TSensor>>::new();
    let mut vents = Vec::<Vent>::new();

    for rig in &settings.rigs {
        rigs.push(Rig::new(rig));
    }

    for s in &settings.sensors {
        sensors.push(Rc::new(TSensor::new(s)));
    }

    for v in &settings.vents {
        vents.push(Vent::new(v, &sensors));
    }

    loop {

        let mut gpu_temps = Vec::<isize>::new();
        for r in &mut rigs {
            if let Some(mut res) = r.handle() {
                gpu_temps.append(&mut res.temp);
            }
            // println!("RESULT {:?}", h);
        }

        // Ventilation stuff
        for v in &mut vents {
            v.handle(&gpu_temps);
        }

        thread::sleep(Duration::from_millis(1000));
    }
}
