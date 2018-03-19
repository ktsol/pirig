#[derive(Debug, Clone, Deserialize)]
pub struct TempSensorCfg {
    pub id: String,
    pub gpio: u8,
}

#[derive(Clone, Debug, Deserialize)]
pub struct VentCfg {
    pub sensors: Vec<String>,
    pub sensors_temp_on: isize,
    pub sensors_temp_off: isize,
    pub rig_temp_on: isize,
    pub rig_temp_off: isize,
    pub gpio: u8,
}

#[derive(Debug, Deserialize)]
pub struct RigCfg {
    pub uri: String,
    pub gpio_power: u8,
    pub gpio_switch: u8,
    pub critical_gpu_temp: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub sensors: Vec<TempSensorCfg>,
    pub vents: Vec<VentCfg>,
    pub rigs: Vec<RigCfg>,
}
