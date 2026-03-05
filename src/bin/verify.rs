use std::env;
use std::os::unix::io::AsRawFd;
use std::{thread, time::Duration};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: verify-keyer <serial-port-path>");
        println!("Example: verify-keyer /dev/ttys005");
        return Ok(());
    }

    let port_path = &args[1];
    println!("Opening port: {}", port_path);

    // Open the serial port for reading (the slave side)
    let file = std::fs::File::open(port_path)?;
    let fd = file.as_raw_fd();

    println!("-----------------------------------------");
    println!("Monitoring signals on: {}", port_path);
    println!("Press Ctrl+C to exit.");
    println!("-----------------------------------------");

    let mut last_status: i32 = -1;

    loop {
        let mut status: i32 = 0;
        // TIOCMGET: Get the status of modem bits
        unsafe {
            libc::ioctl(fd, libc::TIOCMGET, &mut status);
        }

        if status != last_status {
            let rts = (status & libc::TIOCM_RTS) != 0;
            let cts = (status & libc::TIOCM_CTS) != 0;
            let dtr = (status & libc::TIOCM_DTR) != 0;
            let dsr = (status & libc::TIOCM_DSR) != 0;

            println!(
                "Status changed: [RTS: {}] [CTS: {}] [DTR: {}] [DSR: {}]",
                if rts { "HIGH" } else { "LOW " },
                if cts { "HIGH" } else { "LOW " },
                if dtr { "HIGH" } else { "LOW " },
                if dsr { "HIGH" } else { "LOW " }
            );
            last_status = status;
        }

        // 10ms polling is enough for visual verification
        thread::sleep(Duration::from_millis(10));
    }
}
