use device_query::{DeviceQuery, DeviceState, Keycode};
use log::{debug, error, info, trace};
use std::{thread, time::Duration};

#[cfg(unix)]
use std::os::unix::io::AsRawFd;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let device_state = DeviceState::new();

    // 1. Initialize the virtual/physical port based on the OS
    let mut port = setup_port()?;

    info!("-----------------------------------------");
    info!("CW Keyer Active!");
    info!("Press 'Z' for DIT (RTS)");
    info!("Press 'X' for DAH (CTS/DTR)");
    info!("Press 'Esc' to exit.");
    info!("-----------------------------------------");

    let mut dit_active = false;
    let mut dah_active = false;

    loop {
        let keys = device_state.get_keys();
        if !keys.is_empty() {
            trace!("Keys currently down: {:?}", keys);
        }

        // Check for Exit
        if keys.contains(&Keycode::Escape) {
            info!("Exiting...");
            break;
        }

        // Logic for DIT (Z key -> RTS)
        let z_down = keys.contains(&Keycode::Z);
        if z_down != dit_active {
            dit_active = z_down;
            debug!(
                "Key Z {}, setting RTS to {}",
                if dit_active { "DOWN" } else { "UP" },
                dit_active
            );
            port.set_rts(dit_active)?;
        }

        // Logic for DAH (X key -> CTS/DTR)
        let x_down = keys.contains(&Keycode::X);
        if x_down != dah_active {
            dah_active = x_down;
            debug!(
                "Key X {}, setting DTR to {}",
                if dah_active { "DOWN" } else { "UP" },
                dah_active
            );
            port.set_cts(dah_active)?;
        }

        // 2ms sleep provides ~500Hz polling rate (very low latency for CW)
        // while keeping CPU usage near 0%.
        thread::sleep(Duration::from_millis(2));
    }

    Ok(())
}

// ---------------------------------------------------------
// PORT ABSTRACTION
// ---------------------------------------------------------

trait CwKeyerPort {
    fn set_rts(&mut self, active: bool) -> Result<(), Box<dyn std::error::Error>>;
    fn set_cts(&mut self, active: bool) -> Result<(), Box<dyn std::error::Error>>;
}

// ---------------------------------------------------------
// UNIX IMPLEMENTATION (PTY Master)
// ---------------------------------------------------------
#[cfg(unix)]
struct UnixCwPort {
    _master_fd: nix::pty::PtyMaster,
    slave_fd: std::fs::File,
}

#[cfg(unix)]
fn setup_port() -> Result<Box<dyn CwKeyerPort>, Box<dyn std::error::Error>> {
    use nix::fcntl::OFlag;
    use nix::pty::{grantpt, posix_openpt, unlockpt};
    use std::ffi::CStr;
    use std::os::unix::io::AsRawFd;

    // Open a master pseudoterminal
    let master_fd = posix_openpt(OFlag::O_RDWR | OFlag::O_NOCTTY)?;

    // Grant access to the slave and unlock it
    grantpt(&master_fd)?;
    unlockpt(&master_fd)?;

    // Use libc to get the slave terminal name
    let slave_path = unsafe {
        let name_ptr = libc::ptsname(master_fd.as_raw_fd());
        if name_ptr.is_null() {
            return Err("libc::ptsname failed".into());
        }
        CStr::from_ptr(name_ptr).to_string_lossy().into_owned()
    };

    info!("Unix Mode: Virtual Serial Port created.");
    info!(
        "Connect your Radio App (e.g., fldigi, Thetis) to: {}",
        slave_path
    );

    // Open the SLAVE side for ourselves to set modem bits
    use std::fs::OpenOptions;
    use std::os::unix::fs::OpenOptionsExt;
    let slave_file = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_NOCTTY)
        .open(&slave_path)?;

    Ok(Box::new(UnixCwPort {
        _master_fd: master_fd,
        slave_fd: slave_file,
    }))
}

#[cfg(unix)]
impl CwKeyerPort for UnixCwPort {
    fn set_rts(&mut self, active: bool) -> Result<(), Box<dyn std::error::Error>> {
        use nix::libc::{TIOCMBIC, TIOCMBIS, TIOCM_RTS};
        let request = if active { TIOCMBIS } else { TIOCMBIC };
        let line = TIOCM_RTS;
        debug!(
            "ioctl(TIOCM_RTS) request={} active={}",
            if active { "TIOCMBIS" } else { "TIOCMBIC" },
            active
        );
        let res = unsafe { libc::ioctl(self.slave_fd.as_raw_fd(), request as _, &line) };
        if res == -1 {
            let err = std::io::Error::last_os_error();
            error!("ioctl(TIOCM_RTS) on slave failed: {}", err);
        }
        Ok(())
    }

    fn set_cts(&mut self, active: bool) -> Result<(), Box<dyn std::error::Error>> {
        use nix::libc::{TIOCMBIC, TIOCMBIS, TIOCM_DTR};
        let request = if active { TIOCMBIS } else { TIOCMBIC };
        let line = TIOCM_DTR;
        debug!(
            "ioctl(TIOCM_DTR) request={} active={}",
            if active { "TIOCMBIS" } else { "TIOCMBIC" },
            active
        );
        let res = unsafe { libc::ioctl(self.slave_fd.as_raw_fd(), request as _, &line) };
        if res == -1 {
            let err = std::io::Error::last_os_error();
            error!("ioctl(TIOCM_DTR) on slave failed: {}", err);
        }
        Ok(())
    }
}

// ---------------------------------------------------------
// WINDOWS IMPLEMENTATION (com0com client)
// ---------------------------------------------------------
#[cfg(windows)]
struct WindowsCwPort {
    port: Box<dyn serialport::SerialPort>,
}

#[cfg(windows)]
fn setup_port() -> Result<Box<dyn CwKeyerPort>, Box<dyn std::error::Error>> {
    let port_name = "COM8";
    let port = serialport::new(port_name, 9600).open()?;
    info!(
        "Windows Mode: Connected to {}. Radio should be on linked port.",
        port_name
    );
    Ok(Box::new(WindowsCwPort { port }))
}

#[cfg(windows)]
impl CwKeyerPort for WindowsCwPort {
    fn set_rts(&mut self, active: bool) -> Result<(), Box<dyn std::error::Error>> {
        debug!("Windows: set_rts({})", active);
        self.port.write_request_to_send(active)?;
        Ok(())
    }

    fn set_cts(&mut self, active: bool) -> Result<(), Box<dyn std::error::Error>> {
        debug!("Windows: set_cts({})", active);
        self.port.write_data_terminal_ready(active)?;
        Ok(())
    }
}
