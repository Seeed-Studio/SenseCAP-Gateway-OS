use std::{thread, time::Duration};
use std::process::Command;

use gpio_cdev::{Chip, LineRequestFlags};

// Default values
const DEFAULT_GPIOCHIP: &str = "/dev/gpiochip2";
const DEFAULT_SX1302_POWER_EN: u32 = 0;
const DEFAULT_SX1302_RESET: u32 = 2;
const DEFAULT_SX1261_RESET: u32 = 1;

// UCI configuration paths
const UCI_SX1302_POWER_EN_CHIP: &str = "hardware.hardware.sx1302_power_en_chip";
const UCI_SX1302_POWER_EN_PIN: &str = "hardware.hardware.sx1302_power_en_pin";
const UCI_SX1302_RESET_CHIP: &str = "hardware.hardware.sx1302_reset_chip";
const UCI_SX1302_RESET_PIN: &str = "hardware.hardware.sx1302_reset_pin";
const UCI_SX1261_RESET_CHIP: &str = "hardware.hardware.sx1261_reset_chip";
const UCI_SX1261_RESET_PIN: &str = "hardware.hardware.sx1261_reset_pin";

struct GpioConfig {
    power_en_chip: String,
    power_en_pin: u32,
    reset_chip: String,
    reset_pin: u32,
    sx1261_chip: String,
    sx1261_pin: u32,
}

/// Get UCI configuration value by key
fn get_uci_config(key: &str) -> Option<String> {
    let output = Command::new("uci")
        .arg("get")
        .arg(key)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !value.is_empty() {
                    Some(value)
                } else {
                    None
                }
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

/// Load GPIO configuration from UCI, with fallback to defaults
fn load_gpio_config() -> GpioConfig {
    // SX1302 POWER_EN configuration
    let power_en_chip = get_uci_config(UCI_SX1302_POWER_EN_CHIP)
        .unwrap_or_else(|| DEFAULT_GPIOCHIP.to_string());
    let power_en_pin = get_uci_config(UCI_SX1302_POWER_EN_PIN)
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_SX1302_POWER_EN);

    // SX1302 RESET configuration
    let reset_chip = get_uci_config(UCI_SX1302_RESET_CHIP)
        .unwrap_or_else(|| DEFAULT_GPIOCHIP.to_string());
    let reset_pin = get_uci_config(UCI_SX1302_RESET_PIN)
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_SX1302_RESET);

    // SX1261 RESET configuration
    let sx1261_chip = get_uci_config(UCI_SX1261_RESET_CHIP)
        .unwrap_or_else(|| DEFAULT_GPIOCHIP.to_string());
    let sx1261_pin = get_uci_config(UCI_SX1261_RESET_PIN)
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_SX1261_RESET);

    GpioConfig {
        power_en_chip,
        power_en_pin,
        reset_chip,
        reset_pin,
        sx1261_chip,
        sx1261_pin,
    }
}

fn wait_gpio() {
    thread::sleep(Duration::from_millis(20));
}

fn start(config: &GpioConfig) -> anyhow::Result<()> {
    let mut power_en_chip = Chip::new(&config.power_en_chip)?;
    let mut reset_chip = Chip::new(&config.reset_chip)?;
    let mut sx1261_chip = Chip::new(&config.sx1261_chip)?;

    let h_power = power_en_chip
        .get_line(config.power_en_pin)?
        .request(LineRequestFlags::OUTPUT, 0, "radio_init:power_en")?;
    let h_sx1302 = reset_chip
        .get_line(config.reset_pin)?
        .request(LineRequestFlags::OUTPUT, 0, "radio_init:sx1302_reset")?;
    let h_sx1261 = sx1261_chip
        .get_line(config.sx1261_pin)?
        .request(LineRequestFlags::OUTPUT, 0, "radio_init:sx1261_reset")?;

    println!(
        "CoreCell power enable via {} line {}...",
        config.power_en_chip, config.power_en_pin
    );
    h_power.set_value(1)?;
    wait_gpio();

    println!("CoreCell reset via {} line {}...", config.reset_chip, config.reset_pin);
    h_sx1302.set_value(1)?;
    wait_gpio();
    h_sx1302.set_value(0)?;
    wait_gpio();

    println!("SX1261 reset via {} line {}...", config.sx1261_chip, config.sx1261_pin);
    h_sx1261.set_value(0)?;
    wait_gpio();
    h_sx1261.set_value(1)?;
    wait_gpio();

    Ok(())
}

fn stop(config: &GpioConfig) -> anyhow::Result<()> {
    let mut power_en_chip = Chip::new(&config.power_en_chip)?;
    let mut reset_chip = Chip::new(&config.reset_chip)?;
    let mut sx1261_chip = Chip::new(&config.sx1261_chip)?;

    let h_power = power_en_chip
        .get_line(config.power_en_pin)?
        .request(LineRequestFlags::OUTPUT, 0, "radio_stop:power_en")?;
    let h_sx1302 = reset_chip
        .get_line(config.reset_pin)?
        .request(LineRequestFlags::OUTPUT, 0, "radio_stop:sx1302_reset")?;
    let h_sx1261 = sx1261_chip
        .get_line(config.sx1261_pin)?
        .request(LineRequestFlags::OUTPUT, 0, "radio_stop:sx1261_reset")?;

    println!(
        "CoreCell power disable via {} line {}...",
        config.power_en_chip, config.power_en_pin
    );
    h_power.set_value(0)?;
    wait_gpio();

    h_sx1261.set_value(0)?;
    wait_gpio();

    println!("CoreCell reset via {} line {}...", config.reset_chip, config.reset_pin);
    h_sx1302.set_value(0)?;
    wait_gpio();

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Load GPIO configuration from UCI
    let config = load_gpio_config();

    if args.len() == 1 || (args.len() == 2 && args[1] == "start") {
        start(&config)
    } else if args.len() == 2 && args[1] == "stop" {
        stop(&config)
    } else {
        start(&config)
    }
}
