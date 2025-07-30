# `bar_daemon` -- A Daemon For Status Bars
A daemon that can be queried for specific values, and set values (With notifications for some), can also run as a listener cwhich will be sent all of the values in JSON format whenever a value is updated (Certain values are polled, e.g battery, ram).

Notifies on the change of values, and can be queried for the icon of a particular entry, given its type and value.

Intended for use with a status bar, reduces the amount of values which need to be polled for.


## Usage
Listen for changes/polled values
```
bar_daemon listen
```

Start daemon
```
bar_daemon daemon
```

Get Volume Percent
```
bar_daemon get volume percent
bar_daemon get v p
bar_daemon get vol per
```

Get Battery Time
```
bar_daemon get battery time
bar_daemon get bat time
bar_daemon get bat t
```

Get Battery Icon
```
bar_daemon get battery icon
bar_daemon get bat i
```

Set Fan Speed
```
bar_daemon set fan profile Balanced
bar_daemon set fanprofile profile next
bar_daemon set fan p prev
```

Get All (Responds with an Enum of all the tuples)
```
bar_daemon get
bar_daemon get all
```

Use `bar_daemon help` or `bar_daemon <COMMAND> help` to get more info about usage


### Requirements

* `wpctl` (Pipewire) for volume control
* `brightnessctl` for keyboard and monitor brightness control (Devices are set manually in the code)
* `bluetoothctl` for bluetooth control
* `free` for viewing memory usage
* `acpi` for viewing battery stats
* `asusctl` for fan-speed control

