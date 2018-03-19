use reqwest;
use toml;

use gpio_sensors::gpio::GpioPin;
use gpio_sensors::gpio::gpio_pin_new;

use core::RigCfg;

use std::error::Error;
use std::fmt;
use std::time::{Duration, Instant};
use std::thread;

/// Allowed boot time  untill service start
const BOOT_WAIT: u64 = 300; //300;
/// Allowed wait period for full power off after power switch click
const POWER_OFF_WAIT: u64 = 120;
/// Minimun time to be in power off state
const POWER_OFF: u64 = 180;
/// Wait until error resolved
const ERR_RESOLVE_WAIT: u64 = 30;

#[derive(Debug, Deserialize)]
pub struct RigCheckResult {
    pub hostname: String,
    pub temp: Vec<isize>,
    pub service: bool,
    pub hw_errors: bool,
}

#[derive(Debug)]
pub enum RigState {
    /// It is working now
    On,
    /// Rig is on but some errors detected at Instant
    OnErr(Instant),
    /// Rig booting started at Instant
    Boot(Instant),
    /// Rig power off button clicked at Instant
    PowOff(Instant),
    /// Rig power off button pressed (not released) at Instant
    PowOffHard(Instant),
    /// Rig is off since Instant
    Off(Instant),
}

// #[derive(Debug)]
pub struct Rig {
    hostname: String,
    uri: String,
    state: RigState,
    critical_temp: u32,
    pin_power: Box<GpioPin>,
    pin_switch: Box<GpioPin>,
}

impl Rig {
    pub fn new(cfg: &RigCfg) -> Rig {
        Rig {
            hostname: String::from("N/A"),
            uri: cfg.uri.clone(),
            // Possible SHOULD BE OFF
            state: RigState::On,
            critical_temp: cfg.critical_gpu_temp.unwrap_or(85),
            pin_power: gpio_pin_new(cfg.gpio_power as u32)
                .expect(format!("Can not access Rig power pin {}", cfg.gpio_power).as_str()),
            pin_switch: gpio_pin_new(cfg.gpio_switch as u32)
                .expect(format!("Can not access Rig switch pin {}", cfg.gpio_power).as_str()),
        }
    }

    /// Handle all rig processing and checks
    pub fn handle(&mut self) -> Option<RigCheckResult> {
        match self.state {
            RigState::Off(_) => if self.read_power_state() {
                trace!("Turn ON FROM OFF {} {}", self.hostname, self.uri);
                // Should SWITCH TO BOOT state
                self.to_on();
            },
            _ => if !self.read_power_state() {
                self.to_off();
                // no need to do anything right now
                return None;
            },
        }

        let now = Instant::now();
        match self.state {
            RigState::On => match self.request_check() {
                Ok(check) => {
                    self.process_checks(&check);
                    return Some(check);
                }
                Err(err) => {
                    warn!("{} check failed. {}", self.hostname, err);
                    self.to_on_err();
                }
            },
            RigState::OnErr(from) => match self.request_check() {
                Ok(check) => {
                    self.process_checks(&check);
                    return Some(check);
                }
                Err(err) => {
                    warn!("{} check failed. {}", self.hostname, err);
                    if now - from > Duration::from_secs(ERR_RESOLVE_WAIT) {
                        self.to_power_off();
                    }
                }
            },
            RigState::Boot(from) => if now - from > Duration::from_secs(BOOT_WAIT) {
                match self.request_check() {
                    Ok(check) => {
                        self.to_on();
                        self.process_checks(&check);
                        return Some(check);
                    }
                    Err(err) => {
                        warn!("{} check failed. {}", self.hostname, err);
                        self.to_on_err();
                    }
                }
            } else {
                trace!("Wait {} at {} for boot", self.hostname, self.uri);
            },
            RigState::PowOff(from) => if now - from > Duration::from_secs(POWER_OFF_WAIT) {
                self.to_power_off_hard();
            },
            RigState::PowOffHard(_) => if !self.read_power_state() {
                self.to_off();
            },
            RigState::Off(from) => if now - from > Duration::from_secs(POWER_OFF) {
                self.to_on();
            },
        }
        return None;
    }

    fn process_checks(&mut self, res: &RigCheckResult) {
        // Big erros - should turn off
        if res.hw_errors {
            return self.to_power_off();
        }
        for t in &res.temp {
            if t > &(self.critical_temp as isize) {
                return self.to_power_off();
            }
        }
        // Regular errors
        if !res.service {
            self.to_on_err();
        }
    }

    fn request_check(&mut self) -> Result<RigCheckResult, String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(reqwest_err_map)?;

        let result = client
            .get(&self.uri)
            .send()
            .map_err(reqwest_err_map)?
            .text()
            .map_err(reqwest_err_map)?;

        let resp =
            toml::from_str::<RigCheckResult>(&result).map_err(|e| String::from(e.description()));
        trace!("RESPONSE: {:?}", resp);

        return resp.and_then(|r| {
            if r.hostname != self.hostname {
                self.hostname = r.hostname.clone();
            }
            Ok(r)
        });
    }

    fn read_power_state(&mut self) -> bool {
        self.pin_power.read() > 0
    }

    fn to_power_off(&mut self) {
        match self.state {
            RigState::On | RigState::OnErr(_) | RigState::Boot(_) => {
                let offres = self.switch_pin_hight().and_then(|_| {
                    thread::sleep(Duration::from_millis(250));
                    self.switch_pin_low()
                });

                match offres {
                    Ok(_) => {
                        warn!("{} powering OFF", self.hostname);
                        self.state = RigState::PowOff(Instant::now());
                    }
                    Err(e) => error!(
                        "can not power switch pin for PowerOff for {}. {}",
                        self.hostname, e
                    ),
                }
            }
            _ => warn!(
                "can not PowerOff from {:?} for {}",
                self.state, self.hostname
            ),
        }
    }

    fn to_power_off_hard(&mut self) {
        match self.state {
            RigState::PowOffHard(_) => { /* Do nothing same state */ }
            _ => match self.switch_pin_hight() {
                Ok(_) => {
                    warn!("{} powering OFF HARD", self.hostname);
                    self.state = RigState::PowOffHard(Instant::now());
                }
                Err(e) => error!(
                    "can not set pin hight for PowerOffHard for {}. {}",
                    self.hostname, e
                ),
            },
        }
    }

    fn to_off(&mut self) {
        if let Err(e) = self.switch_pin_low() {
            error!(
                "can not set pin low for Off state for {}. {}",
                self.hostname, e
            );
        }

        if let RigState::Off(_) = self.state {
            return;
        }

        info!("{} is OFF", self.hostname);
        self.state = RigState::Off(Instant::now());
    }

    fn to_on(&mut self) {
        match self.state {
            RigState::Boot(_) => {
                info!("{} is ON", self.hostname);
                self.state = RigState::On;
            }
            RigState::On => { /* Do nothing same state */ }
            RigState::Off(_) => {
                // click
                if self.read_power_state() {
                    info!("{} booting", self.hostname);
                    self.state = RigState::Boot(Instant::now());
                } else {
                    error!("{} can not start boot", self.hostname);
                }
            }
            _ => warn!("can not On from {:?} for {}", self.state, self.hostname),
        }
    }

    fn to_on_err(&mut self) {
        match self.state {
            RigState::On => {
                debug!("state to OnErr for {}", self.hostname);
                self.state = RigState::OnErr(Instant::now());
            }
            _ => warn!("can not OnErr from {:?} for {}", self.state, self.hostname),
        }
    }

    fn switch_pin_hight(&mut self) -> Result<(), String> {
        self.pin_switch.set_high();
        debug!("{}: GPIO to hight", self.hostname);
        Ok(())
    }

    fn switch_pin_low(&mut self) -> Result<(), String> {
        self.pin_switch.set_low();
        debug!("{}: GPIO to low", self.hostname);
        Ok(())
    }
}

impl fmt::Debug for Rig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Rig (\"{}\" {})", self.hostname, self.uri)
    }
}

fn reqwest_err_map(e: reqwest::Error) -> String {
    return format!("REQWEST: {}", e.description());
    // return format!("{:?}", e);
}
