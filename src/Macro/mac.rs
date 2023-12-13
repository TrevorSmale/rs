// adafruit macropad test in rust
// modified from https://github.com/KOBA789/rusty-keys
// replace GT with greater than
// replace LT with less than
// youtube don't like angle brackets
// source code -- http://crus.in/codes/macropad_rust.txt

#![no_main]
#![no_std]

use bsp::hal::{self, usb::UsbBus};
use hal::gpio::DynPin;
use cortex_m::prelude::*;
use cortex_m_rt::entry;
use defmt_rtt as _;
use embedded_hal::digital::v2::InputPin;
use embedded_time::duration::Extensions as _;
use hal::pac;
use heapless::Deque;
use panic_probe as _;
use rp_pico as bsp;
use usb_device as usbd;
use usbd::{
    class_prelude::UsbBusAllocator,
    device::{UsbDeviceBuilder, UsbVidPid},};

use usbd_hid::{
    descriptor::{KeyboardReport, SerializedDescriptor},
    hid_class::{
        HIDClass, HidClassSettings, HidCountryCode, HidProtocol, HidSubClass, ProtocolModeConfig,},};

#[defmt::panic_handler]
fn panic() -GT ! {
    cortex_m::asm::udf()}

pub fn exit() -GT ! {
    loop {
        cortex_m::asm::bkpt();}}

#[entry]
fn main() -GT ! {
    let mut p = pac::Peripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(p.WATCHDOG);
    let clocks = hal::clocks::init_clocks_and_plls(
        bsp::XOSC_CRYSTAL_FREQ,
        p.XOSC,
        p.CLOCKS,
        p.PLL_SYS,
        p.PLL_USB,
        &mut p.RESETS,
        &mut watchdog,).ok().unwrap();
        
    let timer = hal::Timer::new(p.TIMER, &mut p.RESETS);

    let bus = UsbBus::new(
        p.USBCTRL_REGS,
        p.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut p.RESETS,);
        
    let bus_allocator = UsbBusAllocator::new(bus);
    let vid_pid = UsbVidPid(0x6666, 0x0789);
    let mut hid = HIDClass::new_with_settings(
        &bus_allocator,
        KeyboardReport::desc(),
        10,
        HidClassSettings {
            subclass: HidSubClass::NoSubClass,
            protocol: HidProtocol::Keyboard,
            config: ProtocolModeConfig::ForceReport,
            locale: HidCountryCode::NotSupported,},);
            
    let mut dev = UsbDeviceBuilder::new(&bus_allocator, vid_pid)
        .manufacturer("KOBA789")
        .product("RustyKeys")
        .serial_number("789")
        .build();

    let sio = hal::Sio::new(p.SIO);
    let pins = bsp::Pins::new(p.IO_BANK0, p.PADS_BANK0, sio.gpio_bank0, &mut p.RESETS);   
    
    let pinarray: [DynPin; 12] = [
      pins.gpio1.into_pull_up_input().into(),
      pins.gpio2.into_pull_up_input().into(),
      pins.gpio3.into_pull_up_input().into(),
      pins.gpio4.into_pull_up_input().into(),
      pins.gpio5.into_pull_up_input().into(),
      pins.gpio6.into_pull_up_input().into(),
      pins.gpio7.into_pull_up_input().into(),
      pins.gpio8.into_pull_up_input().into(),
      pins.gpio9.into_pull_up_input().into(),
      pins.gpio10.into_pull_up_input().into(),
      pins.gpio11.into_pull_up_input().into(),
      pins.gpio12.into_pull_up_input().into(),];

    let mut scan_countdown = timer.count_down();
    scan_countdown.start(10.milliseconds());

    let mut macro_queue = Deque::_LT_KeyboardReport, 32_GT_::new();

    loop {
        dev.poll(&mut [&mut hid]);
        if scan_countdown.wait().is_ok() {
            if let Some(report) = macro_queue.pop_front() { // check if there is something to send to usb
                hid.push_input(&report).ok();
            } else {  // otherwise scan the buttons
                let mut keycodes = [0u8; 6];
                let offset = 4;  // this translates to 'a'
                for (i, pin) in pinarray.iter().enumerate(){
                    if pin.is_low().unwrap() {
                        let index = i + offset as usize;
                        keycodes[0] = index as u8;
                    }
                    let report = KeyboardReport{
                        modifier:0,
                        reserved:0,
                        leds:0,
                        keycodes,
                    };
                    macro_queue.push_back(report).unwrap();}}
                
        // drop received data
        hid.pull_raw_output(&mut [0; 64]).ok();}}}