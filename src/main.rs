extern crate ctrlc;
#[macro_use] extern crate log;
extern crate pyo3;
extern crate rppal;
extern crate failure;

use failure::Error;
use rppal::gpio::{Gpio, Level, Mode, PullUpDown};

type Result<T> = std::result::Result<T, Error>;

/// Wrapper for Python picamera library
mod picamera {
    use pyo3::prelude::*;

    pub struct Module<'a>(&'a PyModule);

    /// Wrapper for Python PiCamera object
    pub struct PiCamera<'a>(&'a PyObjectRef);

    impl<'a> PiCamera<'a> {
        pub fn new(i_module: &'a Module) -> PyResult<PiCamera<'a>> {
            i_module.0.call("PiCamera", (), ()).map(PiCamera)
        }

        pub fn start_preview(&self) -> PyResult<&PyObjectRef> {
            self.0.call_method("start_preview", (), ())
        }

        pub fn capture<P: AsRef<str>>(&self, i_filepath: P) -> PyResult<&PyObjectRef> {
            self.0.call_method("capture", i_filepath.as_ref(), ())
        }

        pub fn stop_preview(&self) -> PyResult<&PyObjectRef> {
            self.0.call_method("stop_preview", (), ())
        }
    }

    impl<'a> Drop for PiCamera<'a> {
        fn drop(&mut self) {
            self.0.call_method("close", (), ()).unwrap();
        }
    }

    pub fn import<'a>(py: &'a Python) -> PyResult<Module<'a>> {
        py.import("picamera").map(Module)
    }
}

fn main_app() -> Result<()> {
    // Check for Ctrl-C
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let handler_running = running.clone();
    ctrlc::set_handler(move || {
        handler_running.store(false, std::sync::atomic::Ordering::SeqCst);
    }).unwrap();

    // Start Python interpreter and import python library
    let gil = pyo3::Python::acquire_gil();
    let py = gil.python();
    let picamera = picamera::import(&py).unwrap();
    
    // Get camera handle
    let camera = picamera::PiCamera::new(&picamera).unwrap();

    // Set GPIO to input and pull-up
    let mut gpio = Gpio::new()?;
    gpio.set_mode(17, Mode::Input);
    gpio.set_pullupdown(17, PullUpDown::PullUp);

    // Start out with count of 0
    let mut count = 0usize;

    'outer: loop {
        // Start camera
        camera.start_preview().unwrap();
        println!("pic");

        // FIXME: wait_for_edge or ctrl-c
        'inner: loop {
            if running.load(std::sync::atomic::Ordering::SeqCst) {
                break 'outer;
            }
            if gpio.read(17)? == Level::High {
                break 'inner;
            }
        }

        // Increment count
        count += 1;

        // Take a picture and save to specified file
        camera.capture(format!("/home/pi/deepimage/{}.jpg", count)).unwrap();
        camera.stop_preview().unwrap();
    }

    // Return successful
    Ok(())
}

fn main() {
    std::process::exit(match main_app() {
        Ok(()) => 0,
        Err(e) => {error!("{:?}", e); 1},
    });
}
