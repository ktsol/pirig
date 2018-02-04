extern crate getopts;
extern crate rppal;
#[macro_use]
extern crate serde_derive;
extern crate toml;
extern crate libc;
extern crate gpio_sensors;

use gpio_sensors::dht;
use rppal::gpio::{Gpio, Mode, Level};
use rppal::system::DeviceInfo;


mod conf;


use conf::Settings;
use conf::RigCfg;

use std::thread;
use std::time::{Instant, Duration};

use std::env;
use std::fs::File;
use std::io::Error;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::process::exit;


pub fn read_file(p: &PathBuf) -> Result<String, Error> {
    match File::open(p) {
        Ok(mut f) => {
            let mut s = String::new();
            match f.read_to_string(&mut s) {
                Ok(_) => Ok(s),
                Err(e) => Err(e),
            }
        }
        Err(err) => Err(err),
    }
}


fn print_usage(p: &str) {
    println!("\nUsage: {} CONFIG.toml", p);
}


fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let toml_path = if args.len() > 1 {
        Path::new(&args[1])
    } else {
        print_usage(&program);
        exit(1);
    };

    if !toml_path.exists() || !toml_path.is_file() {
        print_usage(&program);
        exit(1);
    }

    let settings: Settings = match read_file(&toml_path.to_path_buf()) {
        Ok(s) => match toml::from_str(&s) {
            Ok(c) => c,
            Err(e) => {
                print_usage(&program);
                println!("ERROR parsing TOML file {:?}", e);
                exit(1);
            }
        }
        Err(e) => {
            print_usage(&program);
            println!("ERROR reading TOML file {:?}", e);
            exit(1);
        }
    };

    match DeviceInfo::new() {
        Ok(di) => {
            println!("Model: {} (SoC: {})", di.model(), di.soc());
        }
        Err(e) => {
            println!("ERROR {}", e)
        }
    }

    println!("SETTINGS {:?}", settings);
    application(settings);
    exit(0);
}

fn application(settings:Settings) {

    let mut gpio = Gpio::new().unwrap();
    // Pin::new()
    for rig in &settings.rigs {
        // gpio.set_mode(rig.gpio_power, Mode::Input);
        // gpio.set_mode(rig.gpio_switch, Mode::Output);
    }

    let mut count: usize = 0;
    let mut sensor = dht::DhtSensor::new(27, dht::DhtType::DHT11).unwrap();
    
    while true {
        for rig in &settings.rigs {
            match gpio.read(rig.gpio_power) {
                Ok(v) => println!("READ {}", v),
                Err(e) => println!("Can not read {}", e)
            }

            // if count % 2 == 0 {
            //     gpio.write(rig.gpio_switch, Level::High)
            // } else {
            //     gpio.write(rig.gpio_switch, Level::Low)
            // }



                // let v = sensor.read_optimistic();
                // println!(" {}C {}%", v.temperature(), v.humidity());
                // let ht = sensor.read_until(0, 2);
               
                let ht = sensor.read();
                if let Ok(v) = ht {
                    println!(" {}C {}%", v.temperature(), v.humidity());
                } else {
                    println!("SENSOR {:?}", ht);
                }



        }

        count += 1;
        thread::sleep(Duration::from_millis(500));
    }
}