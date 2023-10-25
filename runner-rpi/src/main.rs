use std::{env, process::Command};

use anyhow::{bail, Context};

pub fn main() -> anyhow::Result<()> {
    let Some(bin) = env::args().nth(1) else {
        bail!("No binary provided");
    };

    eprintln!("Killing process");

    Command::new("ssh")
        .arg("pi@mate.local")
        .arg("sudo pkill --signal SIGINT mate-exec && sleep 0.5 ; sudo pkill gst-launch-1.0")
        .spawn()
        .context("Spawn ssh")?
        .wait()
        .context("Wait on ssh")?;

    eprintln!();

    eprintln!("Uploading");

    let rst = Command::new("scp")
        .arg("./detect_cameras.sh")
        .arg("pi@mate.local:~/mate/detect_cameras.sh")
        .spawn()
        .context("Spawn scp")?
        .wait();

    let rst = Command::new("scp")
        .arg("./setup_camera.sh")
        .arg("pi@mate.local:~/mate/setup_camera.sh")
        .spawn()
        .context("Spawn scp")?
        .wait()
        .and(rst);

    let rst = Command::new("scp")
        .arg("./robot/forward_motor_data.csv")
        .arg("pi@mate.local:~/forward_motor_data.csv")
        .spawn()
        .context("Spawn scp")?
        .wait()
        .and(rst);

    let rst = Command::new("scp")
        .arg("./robot/reverse_motor_data.csv")
        .arg("pi@mate.local:~/reverse_motor_data.csv")
        .spawn()
        .context("Spawn scp")?
        .wait()
        .and(rst);

    let status = Command::new("scp")
        .arg(bin)
        .arg("pi@mate.local:~/mate/mate-exec")
        .spawn()
        .context("Spawn scp")?
        .wait()
        .and(rst)
        .context("Wait on scp")?;

    if status.success() {
        eprintln!("Upload success!");
    } else {
        bail!("Upload failed: {status}")
    }
    eprintln!();

    eprintln!("Running binary");

    let status = Command::new("ssh")
        .arg("pi@mate.local")
        .arg("sudo ~/mate/mate-exec")
        .spawn()
        .context("Spawn ssh")?
        .wait()
        .context("Wait on ssh")?;

    if status.success() {
        eprintln!("Remote run success!");
    } else {
        bail!("Remote run failed: {status}")
    }

    Ok(())
}
