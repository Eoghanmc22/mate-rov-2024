#!/bin/bash

echo "Uploading to Raspberry Pi"

rsync -avPz -e ssh  ./detect_cameras.sh ./setup_camera.sh ./robot/motor_data.csv ./robot/robot.toml $1 pi@mate.local:~/mate/ &&
  ssh pi@mate.local "journalctl -u mate --all --follow -n0 & cd ~/mate/ ; sudo systemctl stop mate ; sleep 0.5 ; rm ./mate ; mv ./$(basename $1) ./mate ; sudo systemctl start mate & cp -p mate ./$(basename $1)"
