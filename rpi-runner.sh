#!/bin/bash

ssh pi@mate.local "sudo pkill --signal SIGINT mate-exec && sleep 0.5 ; sudo pkill gst-launch-1.0"

echo "Uploading new"

scp ./detect_cameras.sh pi@mate.local:~/mate/detect_cameras.sh
scp ./setup_cameras.sh pi@mate.local:~/mate/setup_cameras.sh
scp ./robot/motor_data.csv pi@mate.local:~/mate/motor_data.csv
scp ./robot/robot.toml pi@mate.local:~/mate/robot.toml
scp $1 pi@mate.local:~/mate/mate-exec && ssh pi@mate.local "cd mate && sudo ./mate-exec"
