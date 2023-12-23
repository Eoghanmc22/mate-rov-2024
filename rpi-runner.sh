#!/bin/bash

echo "Uploading to Raspberry Pi"

rsync -avP -e ssh ./detect_cameras.sh ./setup_camera.sh ./robot/motor_data.csv ./robot/robot.toml $1 pi@mate.local:~/mate/ &&
  ssh pi@mate.local "sudo pkill --signal SIGINT $(basename $1) && sleep 0.75 ; sudo pkill $(basename $1) ; sudo pkill gst-launch-1.0 ; cd mate && sudo ./$(basename $1)"
