use gpio_sensors::gpio::GpioPin;
use gpio_sensors::gpio::gpio_pin_new;

use core::VentCfg;
use sensor::TSensor;

use std::fmt;
use std::rc::Rc;

pub struct Vent {
    cfg: VentCfg,
    gpio: Box<GpioPin>,
    sensors: Vec<Rc<TSensor>>,
}

impl Vent {
    pub fn new(cfg: &VentCfg, sensors: &Vec<Rc<TSensor>>) -> Vent {
        let ss: Vec<Rc<TSensor>> = sensors
            .into_iter()
            .filter(|s| cfg.sensors.contains(s.id()))
            .cloned()
            .collect();

        Vent {
            cfg: cfg.clone(),
            sensors: ss,
            gpio: gpio_pin_new(cfg.gpio as u32)
                .expect(format!("Can not access Vent pin {}", cfg.gpio).as_str()),
        }
    }

    pub fn handle(&mut self, gpus_temp: &Vec<isize>) {
        //let mut temps = Vec::<isize>::new();
        let mut tmax = 0;

        for s in &mut self.sensors {
            match Rc::<TSensor>::get_mut(s)
                .ok_or(String::from("Can not get mutable access"))
                .and_then(|s| s.temperature())
            {
                Ok(t) => if t > tmax {
                    tmax = t;
                },
                Err(e) => error!("Can not get temperature for {} {}", s.id(), e),
            }
        }

        let gmax = gpus_temp.into_iter().cloned().max().unwrap_or(0);
        if tmax > self.cfg.sensors_temp_on || gmax > self.cfg.rig_temp_on {
            self.on();
            // Must not perform off check is we are ON now
            return;
        }

        if tmax < self.cfg.sensors_temp_off || gmax < self.cfg.rig_temp_off {
            self.off();
        }
    }

    fn on(&mut self) {
        self.gpio.set_high();
        info!("Vent pin {} ON", self.cfg.gpio);
    }

    fn off(&mut self) {
        self.gpio.set_low();
        info!("Vent pin {} OFF", self.cfg.gpio);
    }
}

impl fmt::Debug for Vent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Vent (pin:{})", self.cfg.gpio)
    }
}
