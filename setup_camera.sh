#!/bin/bash

if v4l2-ctl -d $1 -l | grep focus; then
	v4l2-ctl -d $1 --set-ctrl=focus_automatic_continuous=0
	v4l2-ctl -d $1 --set-ctrl=focus_absolute=0
fi
