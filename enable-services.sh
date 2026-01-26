#!/bin/bash

if [ "$EUID" -ne 0 ]; then
  echo "Please run this script as root."
  exit 1
fi

SERVICE_DIR="/usr/local/lib/systemd/system/"

if [ ! -f "$SERVICE_DIR"/nvfans.service ] && \
    [ ! -f "$SERVICE_DIR"/nvfans-sleep.service ] && \
    [ ! -f "$SERVICE_DIR"/nvfans-resume.service ]; then

    echo "Service files do not exist!"
    echo "Please install first!"
    exit 1
fi

echo "Checking if services are running"
echo

NVFANS_STATUS=$(systemctl is-active nvfans)
NVFANS_SLEEP_STATUS=$(systemctl is-active nvfans-sleep)
NVFANS_RESUME_STATUS=$(systemctl is-active nvfans-resume)

if [ "$NVFANS_STATUS" = "active" ]; then
    echo "nvfans is currently active."
    echo "Disabling nvfans"
    systemctl disable --now nvfans
    echo 
fi

if [ "$NVFANS_SLEEP_STATUS" = "active" ]; then
    echo "nvfans-sleep is currently active."
    echo "Disabling nvfans-sleep"
    systemctl disable --now nvfans-sleep
    echo
fi

if [ "$NVFANS_RESUME_STATUS" = "active" ]; then
    echo "nvfans-resume is currently active."
    echo "Disabling nvfans-resume"
    systemctl disable --now nvfans-resume
    echo
fi

echo "Enabling services"
systemctl enable --now nvfans-sleep nvfans-resume nvfans
echo

echo "Finished enabling services. Now reloading the daemons"
systemctl daemon-reload

echo "Done!"
