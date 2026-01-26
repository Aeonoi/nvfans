# nvfans

Dynamic and auto fan control daemon for ThinkPads that focuses on quietness and coolness.

This was originally a port and was heavily inspired by [zcfan](https://github.com/cdown/zcfan/tree/master).

## Features

TODO

## Usage

nvfans reads all temperature sensors present on the system.
Below is the default config:

```
thinkpad_acpi fan level      | Range low threshold      (C) | Range high threshold      (C) |
-----------------------------|------------------------------|------------------------------|
0                            | 0                            | 60                           |
2                            | 60                           | 75                           |
3                            | 75                           | 80                           |
4                            | 80                           | 85                           |
5                            | 85                           | 90                           |
7                            | 90                           | 100                          | 
```

The temperature threshold is done via the average of the temperature history. It will accumulate the last 30 seconds' temperature and is averaged out. This helps prevent issues with spikes along with unnecessary fan spin ups. It will try to prevent fan level 7 spin ups as much as possible.

To override these defaults, you can place a file at `/etc/nvfans.conf`. Example:
```
    90,100,7
    85,90,6
    80,85,5
    75,80,4
    70,75,3
    65,70,2
    60,65,1
    0,60,0
```

## Comparison with thinkfan and zcfan

`nvfans` was created because I wanted a dynamic fan control system that tries to handle the CPU load spikes that comes with stronger CPUs and uses the `performance` governor. I also wanted to prevent as much fan noises as possible without entirely relying on fan trips. 

I also wanted a fan control GUI available for ThinkPads. 

Compare your use case and determine which tool fits your need.

BTW, `nvfans` is written in Rust.

## Installation and usage

Make sure that thinkpad_acpi is enabled:

* At runtime: `rmmod thinkpad_acpi && modprobe thinkpad_acpi fan_control=1`
* By default: `echo options thinkpad_acpi fan_control=1 > /etc/modprobe.d/99-fancontrol.conf`


1. Run `make # Compiles the code ` 
2. Run `sudo make install # Installs the compiled code and all other necessary files`

Optional:

If you want to have the services enabled for you:
```
chmod +x enable-services.sh
sudo ./enable-services.sh
```

## Disclaimer

This has only ever been tested on only one machine. Thus, there is no warranty for any mishaps 
that may occur.
