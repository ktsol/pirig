use core::TempSensorCfg;
use gpio_sensors::dht::{DhtSensor, DhtType};

use std::error::Error;


#[derive(Debug)]
pub struct TSensor {
    id: String,
    pin: u8,
    dht: DhtSensor
}

impl TSensor {
    pub fn new(cfg: &TempSensorCfg) -> TSensor {
        TSensor {
            id: cfg.id.clone(),
            pin: cfg.gpio,
            dht: DhtSensor::new(cfg.gpio, DhtType::DHT11)
            .expect(format!("Can not create DHT for pin {}", cfg.gpio).as_str() )
        }
    }

    pub fn id(&self) -> &String {
        &self.id
    }

    pub fn temperature(&mut self) -> Result<isize, String> {
        self.dht.read()
        .map(|t| t.temperature() as isize)
        .map_err(|e| String::from(e.description()))
    }
}
