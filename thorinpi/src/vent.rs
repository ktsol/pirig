use gpio_sensors::gpio::GpioPin;
use gpio_sensors::gpio::gpio_pin_new;

use core::VentCfg;
use sensor::TSensor;

use std::fmt;
use std::rc::Rc;
use std::cell::RefCell;
use std::error::Error;

pub struct Vent {
    cfg: VentCfg,
    gpio: Box<GpioPin>,
    gpio_high: bool,
    sensors: Vec<Rc<RefCell<TSensor>>>,
}

impl Vent {
    pub fn new(cfg: &VentCfg, sensors: &Vec<Rc<RefCell<TSensor>>>) -> Vent {
        let ss: Vec<Rc<RefCell<TSensor>>> = sensors
            .into_iter()
            .filter(|s| cfg.sensors.contains(s.borrow().id()))
            .map(|v| v.clone())
            .collect();

        let mut pin = gpio_pin_new(cfg.gpio as u32)
            .expect(format!("Can not access Vent pin {}", cfg.gpio).as_str());
        pin.direction_output(0)
            .expect(format!("Can not set output mode for pin {}", cfg.gpio).as_str());

        Vent {
            cfg: cfg.clone(),
            sensors: ss,
            gpio: pin,
            gpio_high: false,
        }
    }

    pub fn handle(&mut self, gpus_temp: &Vec<isize>) {
        //let mut temps = Vec::<isize>::new();
        let mut tmax = -100;

        for s in &self.sensors {
            match s.try_borrow_mut()
                //.ok_or(String::from("Can not get mutable access"))
                .map_err(|e| String::from(e.description()))
                .and_then(|ref mut s| s.temperature())
            {
                Ok(t) => if t > tmax {
                    tmax = t;
                },
                Err(e) => error!("Can not get temperature for \"{}\" {}", s.borrow().id(), e),
            }
        }

        let gmax = gpus_temp.into_iter().cloned().max().unwrap_or(-100);
        trace!(
            "VENT gpio {} gpu max temp {} sensors max temp {}",
            self.cfg.gpio,
            gmax,
            tmax
        );
        if tmax >= self.cfg.sensors_temp_on || gmax >= self.cfg.rig_temp_on {
            self.on(tmax, gmax);
            // Must not perform off check is we are ON now
            return;
        }

        if tmax <= self.cfg.sensors_temp_off && gmax <= self.cfg.rig_temp_off {
            self.off(tmax, gmax);
        }
    }

    fn on(&mut self, t: isize, gt: isize) {
        if !self.gpio_high {
            info!("Vent pin {} ON -> sensor: {}C, gpu: {}C", self.cfg.gpio, t, gt);
        }
        self.gpio.set_high();
        self.gpio_high = true;
    }

    fn off(&mut self, t: isize, gt: isize) {
        if self.gpio_high {
            info!("Vent pin {} OFF -> sensor: {}C, gpu: {}C", self.cfg.gpio, t, gt);
        }
        self.gpio.set_low();
        self.gpio_high = false;
    }
}

impl fmt::Debug for Vent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Vent (pin:{})", self.cfg.gpio)
    }
}
