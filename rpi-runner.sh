#!/bin/bash

echo "Uploading to Raspberry Pi"

rsync -avPz -e ssh  ./detect_cameras.sh ./setup_camera.sh ./robot/motor_data.csv ./robot/robot.toml $1 pi@mate.local:~/mate/ &&
  ssh pi@mate.local "journalctl -u mate --all --follow -n0 & cd ~/mate/ ; mv ./$(basename $1) robot 2>/dev/null ; sudo systemctl restart mate"
