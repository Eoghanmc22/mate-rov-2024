# Robot Setup

Code currently supports a Raspberry Pi 4 & 5

Flash a micro sd card with a Raspbian Lite 64-bit image using the official tool
when prompted to configure the os
- Enable ssh with a known password or key pair
- Set host name
- Set wifi for internet during setup 
- Disable telemetry (it wont have an internet connection anyways)

Boot the pi
Setup terminal
- If using kitty: `kitty +kitten ssh pi@mate.local` and `sudo apt install kitty-terminfo`
Download device tree overlay https://github.com/bluerobotics/BlueOS/blob/71b0f683595361a178c78b4df6eb416868690c3e/install/overlays/spi0-led.dts
Compile it https://github.com/bluerobotics/BlueOS/blob/71b0f683595361a178c78b4df6eb416868690c3e/install/boards/bcm_27xx.sh#L18C2-L18C2

Install gstreamer: `sudo apt-get install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libgstreamer-plugins-bad1.0-dev gstreamer1.0-plugins-ugly gstreamer1.0-plugins-good gstreamer1.0-tools`

Setup Ethernet link as local-link in nmtui

Add the following to the bottom of `/boot/config.txt`
```
# BEGIN MATE

# Setup spi
dtparam=spi=on

dtoverlay=spi0-led
dtoverlay=spi1-3cs

# Setup i2c
dtparam=i2c_vc=on
dtparam=i2c_arm=on

# Turn off all leds by default
gpio=11,24,25=op,pu,dh

# PWM Output enable, Disarm by default
gpio=37=op,pu,dh

[pi4]

dtoverlay=i2c1,pins_2_3
dtoverlay=i2c4,pins_6_7,baudrate=400000
dtoverlay=i2c6,pins_22_23

[pi5]

dtoverlay=i2c1-pi5,pins_2_3
dtoverlay=i2c3-pi5,pins_6_7,baudrate=400000
dtoverlay=i2c-gpio,i2c_gpio_sda=22,i2c_gpio_scl=23,bus=6

[all]

# END MATE
```

add `i2c-dev` to `/etc/modules`

TODO: Need to tell it the amperage rating of the power supply
Need to define "PSU_MAX_CURRENT" in the bootloader configuration
See:
- https://www.raspberrypi.com/documentation/computers/configuration.html
- https://suptronics.com/Raspberrypi/Power_mgmt/x120x-v1.0_software.html

Create file `/etc/systemd/system/mate.service` with contents
```
[Unit]
Description=MATE
After=multi-user.target

[Service]
ExecStart=/usr/bin/bash -c "cd /home/pi/mate && nice -n -10 ./mate"
ExecStop=/usr/bin/bash -c "kill -SIGINT $MAINPID ; pkill gst-launch-1.0 ; sleep 0.75"
Type=exec
Restart=always

[Install]
WantedBy=multi-user.target
```
Enable the service

Setup passwordless ssh: `ssh-copy-id pi@mate.local``
