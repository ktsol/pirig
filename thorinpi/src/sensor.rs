use core::TempSensorCfg;
use gpio_sensors::dht::{DhtSensor, DhtType};

use std::error::Error;

#[derive(Debug)]
pub struct TSensor {
    id: String,
    pin: u8,
    cached_temp: isize,
    dht: DhtSensor,
}

impl TSensor {
    pub fn new(cfg: &TempSensorCfg) -> TSensor {
        TSensor {
            id: cfg.id.clone(),
            pin: cfg.gpio,
            cached_temp: -100,
            dht: DhtSensor::new(cfg.gpio, DhtType::DHT11)
                .expect(format!("Can not create DHT for pin {}", cfg.gpio).as_str()),
        }
    }

    pub fn id(&self) -> &String {
        &self.id
    }

    pub fn temperature(&mut self) -> Result<isize, String> {
        let res = self.dht
            .read_until(3, 3)
            .map(|t| t.temperature() as isize)
            .map(|t| self.check_temperature(t))
            .map_err(|e| String::from(e.description()));

        trace!("Temperature {:?}", res);

        return res;
    }

    fn check_temperature(&mut self, t: isize) -> isize {
        if self.cached_temp - t > 5 && t == 0 {
            debug!(
                "Temperature issue was {} now {} (O_o) Reset to previous value",
                self.cached_temp, t
            );
        } else {
            self.cached_temp = t;
        }
        self.cached_temp
    }
}
