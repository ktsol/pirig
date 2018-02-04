#[derive(Debug,Deserialize)]
pub struct RigCfg {
    pub gpio_power: u8,
    pub gpio_switch: u8,
}

#[derive(Debug,Deserialize)]
pub struct Settings {
    pub rigs: Vec<RigCfg>,
}
