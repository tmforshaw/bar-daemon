# bar_daemon
A daemon that can send or receive commands and outputs in JSON format.


## Usage
Listen for changes/polled values
```
bar_daemon listen
```

Start daemmon
```
bar_daemon daemon
```

Get Volume Percent
`bar_daemon get volume percent` or `bar_daemon get v p` or `bar_daemon get vol per`

Get Battery Time
`bar_daemon get battery time` `bar_daemon get bat time` or `bar_daemon get bat t`

Use `bar_daemon help` or `bar_daemon <COMMAND> help` to get more info about usage


### Requirements

* `wpctl` (Pipewire) for volume control
* `brightnessctl` for keyboard and monitor brightness control (Devices are set manually in the code)
* `bluetoothctl` for bluetooth control
* `free` for viewing memory usage
* `acpi` for viewing battery stats
* `asusctl` for fan-speed control

