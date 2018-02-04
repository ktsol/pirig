extern crate getopts;
#[macro_use]
extern crate serde_derive;
extern crate tiny_http;
extern crate toml;

use getopts::Options;
use tiny_http::{Server, Response};

use std::env;
use std::error::Error;
use std::fs::{read_dir, File};
use std::io::Error as IoError;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;


pub static HWDIR: &'static str = "/sys/class/hwmon";

#[derive(Debug, Deserialize)]
struct Config {
    service: String,
    port: usize,
    gpus: usize,
}

#[derive(Debug, Serialize)]
struct CheckResult {
    hostname: String,
    temp: Vec<i32>,
    service: bool,
    hw_errors: bool,
}


fn read_config(path: &String) -> Result<String, IoError> {
    let p = PathBuf::from(path);
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


fn print_help(program: &str, opts: Options) {
    let brief = format!("Usage: {} FILE [options]", program);
    print!("{}", opts.usage(&brief));
}


fn run_server(cfg: Config) {
    let server = Server::http(format!("0.0.0.0:{}", cfg.port)).unwrap();
    println!("Server started at port {}", cfg.port);
    for request in server.incoming_requests() {

        /*
        println!(
            "received request! method: {:?}, url: {:?}, headers: {:?}",
            request.method(),
            request.url(),
            request.headers()
        );
        */

        let not_root = request.url() != "/";
        if not_root {
            println!(
                "ERROR REQUEST {} {} {}",
                request.remote_addr(),
                request.method(),
                request.url()
            );
            let response = Response::from_string("FUCK YOU!");
            request.respond(response);

        } else {
            println!(
                "REQUEST {} {} {}",
                request.remote_addr(),
                request.method(),
                request.url()
            );

            let checks = check_all(&cfg);
            let response = Response::from_string(toml::to_string(&checks).unwrap());
            request.respond(response);
        }
    }
}


fn main() {

    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("d", "daemon", "run as daemon");
    opts.optflag("i", "info", "output health check info");
    opts.optflag("h", "help", "print this help menu");
    opts.optopt(
        "c",
        "config",
        "set configuration *.toml file path",
        "config.toml",
    );

    let matches;
    match opts.parse(&args[1..]) {
        Ok(m) => {
            matches = m;
        }
        Err(f) => {
            println!("ERROR: {}\n", f.to_string());
            print_help(&program, opts);
            return;
        }
    };


    let cfg_path = matches.opt_str("c");
    if cfg_path.is_none() {
        print_help(&program, opts);
        return;
    }

    let config_text = read_config(&cfg_path.unwrap());
    if config_text.is_err() {
        println!("ERROR reading config file: {}\n", config_text.unwrap_err());
        print_help(&program, opts);
        return;
    }

    let rcfg = toml::from_str::<Config>(config_text.unwrap().as_str());
    if rcfg.is_err() {
        println!("ERROR reading config: {}\n", rcfg.unwrap_err());
        print_help(&program, opts);
        return;
    }

    let cfg = rcfg.unwrap();

    if matches.opt_present("i") {
        let r = check_all(&cfg);
        println!("{}", toml::to_string(&r).unwrap());
        return;
    }

    // DAEMON
    if matches.opt_present("d") {
        run_server(cfg);
    }
}


fn check_all(cfg: &Config) -> CheckResult {
    let temps = check_temp();

    CheckResult {
        hostname: check_hostname(),
        service: check_service(&cfg.service),
        hw_errors: check_hw_errors(cfg, temps.len()),
        temp: temps,
    }
}


fn check_hw_errors(cfg: &Config, temp_readings_count: usize) -> bool {
    if temp_readings_count != cfg.gpus {
        return false;
    }

    let logs = read_service_logs(&cfg.service);
    if logs.len() == 0 {
        return false;
    }

    // Strange checks here only because I can not define pattern as constant [str]
    let checks = vec![
        logs.contains("WATCHDOG: GPU error"),
        logs.contains("hangs in OpenCL call, exit"),
        logs.contains("GpuMiner kx failed"),
        logs.contains("cannot get current temperature, error"),
        logs.contains("are stopped. Restart attemp"),
        logs.contains("Thread exited with code"),
    ];

    for c in checks {
        if c {
            return true;
        }
    }
    false
}

fn read_service_logs(service: &String) -> String {
    // journalctl -b 0 -n 100 -o cat -eu miner
    Command::new("journalctl")
        .arg("-b 0")
        .arg("-o cat")
        .arg("-n 100")
        .arg("-eu")
        .arg(format!("{}", service))
        .output()
        .map(|cmd| String::from(String::from_utf8_lossy(&cmd.stdout)))
        .unwrap_or(String::new())
}

fn check_temp() -> Vec<i32> {
    let mut res: Vec<i32> = Vec::new();
    res.extend(check_hwmon_temp());
    res.extend(check_nw_temp());
    res
}

fn check_hwmon_temp() -> Vec<i32> {
    let base = PathBuf::from(HWDIR);
    if !base.exists() || !base.is_dir() {
        println!("ERROR: Can not read directory {}", HWDIR);
        return Vec::new();
    }

    read_dir(base)
        .unwrap()
        .map(|r| r.unwrap().path())
        .filter(|ref mut p| if p.exists() {
            let mut pp = p.clone();
            pp.push("name");
            match File::open(pp) {
                Ok(mut f) => {
                    let mut s = String::new();
                    if let Ok(_) = f.read_to_string(&mut s) {
                        s.contains("amdgpu")
                    } else {
                        false
                    }
                }
                Err(_) => false,
            }
        } else {
            false
        })
        .map(|mut p| {
            p.push("temp1_input");
            p
        })
        .filter(|ref p| p.exists())
        .map(|p| File::open(p))
        .filter(|fo| fo.is_ok())
        .map(|fo| {
            let mut s = String::new();
            if let Ok(_) = fo.unwrap().read_to_string(&mut s) {
                s.trim().parse::<i32>()
            } else {
                String::new().parse::<i32>()
            }
        })
        .filter(|r| r.is_ok())
        .map(|r| r.unwrap() / 1000)
        .collect()
}

fn check_nw_temp() -> Vec<i32> {
    let mut res: Vec<i32> = Vec::new();
    for gpu_id in 0..get_nv_gpu_count() {
        let rt = get_nv_temp(gpu_id);
        if rt.is_ok() {
            res.push(rt.unwrap());
        }
    }
    res
}

fn get_nv_temp(gpu_id: usize) -> Result<i32, ()> {
    let rcmd = Command::new("nvidia-smi")
        .arg("--query-gpu=temperature.gpu")
        .arg("--format=csv,noheader")
        .arg("-i")
        .arg(format!("{}", gpu_id))
        .output();
    if let Ok(cmd) = rcmd {
        let out = String::from(String::from_utf8_lossy(&cmd.stdout));
        out.trim().parse::<i32>().map_err(|e| ())
    } else {
        Err(())
    }
}

fn get_nv_gpu_count() -> usize {
    let rcmd = Command::new("nvidia-smi")
        .arg("--query-gpu=count")
        .arg("--format=csv,noheader")
        .arg("-i")
        .arg("0")
        .output();

    if let Ok(cmd) = rcmd {
        let out = String::from(String::from_utf8_lossy(&cmd.stdout));
        out.trim().parse::<usize>().unwrap_or(0)
    } else {
        0
    }
}


fn check_service(name: &String) -> bool {
    let rcmd = Command::new("systemctl")
        .arg("is-active")
        .arg(name)
        .output();

    match rcmd {
        Ok(cmd) => {
            let out = String::from(String::from_utf8_lossy(&cmd.stdout));
            #[cfg(debug_assertions)]
            {
                println!("systemctl is-active {} >> {}", name, out.trim());
            }
            out.trim() == "active"
        }
        Err(e) => {
            println!("ERROR: Can not call systemctl: {}", e);
            false
        }

    }
}


fn check_hostname() -> String {
    match Command::new("hostname").output() {
        Ok(cmd) => String::from(String::from_utf8_lossy(&cmd.stdout).trim()),
        Err(e) => {
            //println!("ERROR: Can not call systemctl: {}", e);
            String::from("undefined")
        }
    }
}
