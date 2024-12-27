// Cargo.toml dependencies:
// rp2040-hal = "0.5.0"
// embedded-hal = "0.2.7"
// cortex-m-rt = "0.7.0"
// panic-halt = "0.2.0"
// max6675 = "0.3.0"
// embedded-sdmmc = "0.6.0"

#![no_std]
#![no_main]

use cortex_m_rt::entry;
use rp2040_hal as hal;
use hal::{
    gpio::{Function, Output, Pin, PushPull},
    pac,
    sio::Sio,
    watchdog::Watchdog,
    clocks::init_clocks_and_plls,
};
use panic_halt as _;
use embedded_hal::digital::v2::OutputPin;
use max6675::Max6675;
use embedded_sdmmc::{Controller, BlockSpi, Mode, Volume, VolumeIdx};

const PROFILE_DURATION: u64 = 3600; // 1 hour
const TEMPPROFILE: [(f32, u32); 5] = [
    (100.0, 600),  // Preheat to 100°C over 10 minutes
    (150.0, 300),  // Raise to 150°C over 5 minutes
    (200.0, 300),  // Raise to 200°C over 5 minutes
    (250.0, 300),  // Reflow peak at 250°C for 5 minutes
    (50.0, 1500),  // Cool down to 50°C over 25 minutes
];

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    // Get everything set up
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let clocks = init_clocks_and_plls(
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    ).unwrap();
    let sio = Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Heating element control pin
    let mut heating_element = pins.gpio25.into_push_pull_output();

    // MAX6675 setup for K-type thermocouple
    let cs = pins.gpio17.into_push_pull_output();
    let sck = pins.gpio18.into_push_pull_output();
    let miso = pins.gpio19.into_pull_up_input();
    let mut max6675 = Max6675::new(cs, sck, miso);

    // SD card setup
    let spi = hal::spi::Spi::<_, _, 8>::new(pac.SPI0);
    let sd_cs = pins.gpio22.into_push_pull_output();
    let sd_spi = BlockSpi::new(spi, sd_cs);
    let mut sd_controller = Controller::new(sd_spi, Default::default());
    let mut volume = sd_controller.get_volume(VolumeIdx(0)).unwrap();

    // Run the reflow profile with logging
    run_profile_with_logging(&mut max6675, &mut heating_element, &mut sd_controller, &mut volume);
}

// Function to execute the temperature profile and log data
fn run_profile_with_logging(
    thermocouple: &mut Max6675,
    heating_element: &mut Pin<Output<PushPull>>,
    sd_controller: &mut Controller<BlockSpi>,
    volume: &mut Volume,
) {
    let mut file = sd_controller.open_file_in_dir(
        volume,
        "/log.txt",
        Mode::ReadWriteCreateOrTruncate,
    ).unwrap();

    for &(target_temp, duration_secs) in TEMPPROFILE.iter() {
        let start_time = hal::timer::Timer::get_counter();
        let end_time = start_time + duration_secs;

        while hal::timer::Timer::get_counter() < end_time {
            let current_temp = thermocouple.read_temperature().unwrap_or(0.0);

            // Log data
            let log_entry = format!(
                "Time: {}s, Target Temp: {:.2}°C, Actual Temp: {:.2}°C\n",
                hal::timer::Timer::get_counter() - start_time,
                target_temp,
                current_temp
            );
            sd_controller.write(file, log_entry.as_bytes()).unwrap();

            // Control heating element
            if current_temp < target_temp {
                heating_element.set_high().unwrap(); // Heat up
            } else {
                heating_element.set_low().unwrap(); // Cool down
            }

            // Small delay to avoid spamming logs
            cortex_m::asm::delay(10_000);
        }
    }

    sd_controller.close_file(file).unwrap();
}